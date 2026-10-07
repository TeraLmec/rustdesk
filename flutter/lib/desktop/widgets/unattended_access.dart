import 'package:flutter/material.dart';
import 'package:get/get.dart';

import '../../common.dart';
import '../../common/shared_state.dart';
import '../../common/widgets/toolbar.dart';
import '../../consts.dart';
import '../../models/model.dart';
import '../../models/platform_model.dart';

void showUnattendedDisplayPicker(FFI ffi) {
  final pi = ffi.ffiModel.pi;
  if (ffi.closed ||
      !(isWindows || isLinux) ||
      ffi.connType != ConnType.defaultConn ||
      ffi.ffiModel.isRefreshing ||
      pi.platformAdditions[kPlatformAdditionsUnattendedAccess] != true ||
      pi.displays.length < 2 ||
      bind.sessionGetUseAllMyDisplaysForTheRemoteSession(
              sessionId: ffi.sessionId) ==
          'Y') {
    return;
  }
  final privacyMode = PrivacyModeState.find(ffi.id).value;
  if (privacyMode.isNotEmpty &&
      !allowDisplaySwitchInPrivacyMode(pi, privacyMode)) {
    return;
  }
  final saved = int.tryParse(
      bind.mainGetPeerOptionSync(id: ffi.id, key: kOptionUnattendedDisplay));
  if (saved != null && saved >= 0 && saved < pi.displays.length) {
    openMonitorInTheSameTab(saved, ffi, pi, updateCursorPos: false);
    return;
  }
  bool remember = false;

  ffi.dialogManager.show((setState, close, context) {
    void select(int index) {
      if (ffi.closed) {
        close();
        return;
      }
      final privacyMode = PrivacyModeState.find(ffi.id).value;
      if (index < 0 ||
          index >= pi.displays.length ||
          (privacyMode.isNotEmpty &&
              !allowDisplaySwitchInPrivacyMode(pi, privacyMode))) {
        close();
        return;
      }
      openMonitorInTheSameTab(index, ffi, pi, updateCursorPos: false);
      if (remember) {
        bind.mainSetPeerOption(
            id: ffi.id, key: kOptionUnattendedDisplay, value: '$index');
      }
      close();
    }

    return CustomAlertDialog(
      title: Text(translate('Select Monitor')),
      content: SizedBox(
        width: 360,
        child: Obx(() => Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                CheckboxListTile(
                  title: Text(translate('Remember this display')),
                  value: remember,
                  onChanged: (value) =>
                      setState(() => remember = value == true),
                ),
                for (var i = 0; i < pi.displays.length; i++)
                  ListTile(
                    title: Text(translate('#{${i + 1}} monitor')),
                    subtitle: Text(
                        '${pi.displays[i].width} × ${pi.displays[i].height}'),
                    onTap: () => select(i),
                  ),
              ],
            )),
      ),
      actions: [dialogButton('Close', onPressed: close)],
      onCancel: close,
    );
  }, tag: 'unattended-display-picker');
}
