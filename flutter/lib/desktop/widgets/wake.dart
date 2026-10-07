import 'dart:convert';

import 'package:flutter/material.dart';

import '../../common.dart';
import '../../consts.dart';
import '../../models/platform_model.dart';

Future<void> showWakeDialog(BuildContext context, String id,
    {required bool wakeAndConnect, required VoidCallback connect}) async {
  final connectNow = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (_) => _WakeDialog(id: id, wakeAndConnect: wakeAndConnect));
  if (connectNow == true && context.mounted) connect();
}

class _WakeDialog extends StatefulWidget {
  final String id;
  final bool wakeAndConnect;
  const _WakeDialog({required this.id, required this.wakeAndConnect});

  @override
  State<_WakeDialog> createState() => _WakeDialogState();
}

class _WakeDialogState extends State<_WakeDialog> {
  final _mac = TextEditingController();
  final _broadcast = TextEditingController(text: '255.255.255.255:9');
  final _helper = TextEditingController();
  final _device = TextEditingController();
  final _key = TextEditingController();
  bool _useHelper = false;
  bool _hasKey = false;
  bool _busy = true;
  String _error = '';
  String _status = 'Loading...';

  Future<dynamic> _request(String action,
      {Map<String, String>? profile}) async {
    final response = jsonDecode(await bind.mainGetCommon(
        key: 'wake:${jsonEncode({
          'action': action,
          'id': widget.id,
          if (profile != null) 'profile': profile
        })}'));
    if (response['ok'] != true) throw Exception(response['error']);
    return response['value'];
  }

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final result = await _request('load');
      if (!mounted) return;
      final profile = result['profile'];
      _mac.text = profile['mac'];
      _broadcast.text = profile['broadcast'];
      _helper.text = profile['helper'];
      _device.text = profile['device'];
      _useHelper = _helper.text.isNotEmpty;
      _hasKey = result['has_key'] == true;
      setState(() => _busy = false);
      if (widget.wakeAndConnect && (_useHelper || _mac.text.isNotEmpty)) {
        await _save(wake: true);
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _error = error.toString();
          _busy = false;
        });
      }
    }
  }

  Future<void> _save({required bool wake}) async {
    setState(() {
      _busy = true;
      _error = '';
      _status = 'Saving...';
    });
    try {
      await _request('save', profile: {
        'mac': _mac.text.trim(),
        'broadcast': _broadcast.text.trim(),
        'helper': _useHelper ? _helper.text.trim() : '',
        'device': _useHelper ? _device.text.trim() : '',
        'key': _useHelper ? _key.text.trim() : '',
      });
      if (!mounted) return;
      if (wake) {
        setState(() => _status = 'Sending wake request...');
        await _request('wake');
        if (!mounted) return;
        showToast(
            translate('Wake request sent. Waiting for the remote device.'));
      }
      Navigator.of(context).pop(wake);
    } catch (error) {
      if (mounted) {
        setState(() {
          _error = error.toString();
          _busy = false;
        });
      }
    }
  }

  @override
  void dispose() {
    for (final controller in [_mac, _broadcast, _helper, _device, _key]) {
      controller.dispose();
    }
    super.dispose();
  }

  Widget _field(String label, TextEditingController controller,
      {bool secret = false, String? hint}) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: TextField(
          controller: controller,
          enabled: !_busy,
          obscureText: secret,
          autocorrect: false,
          enableSuggestions: false,
          decoration:
              InputDecoration(labelText: translate(label), hintText: hint)),
    );
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(translate(
          widget.wakeAndConnect ? 'Wake and connect' : 'Wake settings')),
      content: SizedBox(
          width: 460,
          child: SingleChildScrollView(
              child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(widget.id),
              SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  title: Text(translate('Use a wake helper')),
                  value: _useHelper,
                  onChanged: _busy
                      ? null
                      : (value) => setState(() => _useHelper = value)),
              if (_useHelper) ...[
                _field('Wake helper address', _helper,
                    hint: 'helper.example:21120'),
                _field('Wake device name', _device),
                _field('Wake key', _key,
                    secret: true,
                    hint: _hasKey
                        ? translate('Leave empty to keep the saved key')
                        : null),
              ] else ...[
                _field('Ethernet MAC address', _mac, hint: '02:11:22:33:44:55'),
                _field('Broadcast address and port', _broadcast,
                    hint: '192.168.1.255:9'),
              ],
              Text(translate('wake-setup-tip')),
              TextButton(
                  onPressed: _busy
                      ? null
                      : () async {
                          await bind.mainSetPeerOption(
                              id: widget.id,
                              key: kOptionUnattendedDisplay,
                              value: '');
                          if (mounted) showToast(translate('Successful'));
                        },
                  child: Text(
                      translate('Ask for a display on the next connection'))),
              if (_busy) ...[
                const LinearProgressIndicator(),
                Text(translate(_status))
              ],
              if (_error.isNotEmpty)
                Text(_error,
                    style:
                        TextStyle(color: Theme.of(context).colorScheme.error)),
            ],
          ))),
      actions: [
        TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(translate('Cancel'))),
        TextButton(
            onPressed: _busy ? null : () => _save(wake: false),
            child: Text(translate('Save'))),
        FilledButton(
            onPressed: _busy ? null : () => _save(wake: true),
            child: Text(translate('Wake and connect'))),
      ],
    );
  }
}
