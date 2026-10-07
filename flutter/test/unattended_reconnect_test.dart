import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/widgets/unattended_reconnect.dart';

void main() {
  test(
      'retries boot and session-transition failures, not authentication errors',
      () {
    expect(
        isUnattendedTransientError(
            'Connection Error', 'Remote desktop is offline'),
        isTrue);
    expect(isUnattendedTransientError('Connection Error', 'Reset by the peer'),
        isTrue);
    expect(
        isUnattendedTransientError('Connection Error', 'Socket reset',
            suggested: true),
        isTrue);
    expect(
        isUnattendedTransientError(
            'Connection Error', 'Unattended screen capture is unavailable.',
            suggested: true),
        isFalse);
    expect(isUnattendedTransientError('Connection Error', 'Wrong Password'),
        isFalse);
    expect(
        isUnattendedTransientError(
            'Connection Error', 'Closed manually by the peer'),
        isFalse);
  });

  test('retry delay is capped and stops after three minutes', () {
    expect(unattendedRetryDelay(Duration.zero, 1), 1);
    expect(unattendedRetryDelay(const Duration(seconds: 20), 64), 5);
    expect(unattendedRetryDelay(const Duration(seconds: 179), 5), 1);
    expect(unattendedRetryDelay(const Duration(minutes: 3), 5), isNull);
  });
}
