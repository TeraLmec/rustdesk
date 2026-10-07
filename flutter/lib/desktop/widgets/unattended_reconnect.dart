const unattendedRetryWindow = Duration(minutes: 3);

bool isUnattendedTransientError(String title, String text,
    {bool suggested = false}) {
  if (title != 'Connection Error' || text.startsWith('Unattended ')) {
    return false;
  }
  return suggested ||
      const {
        'Remote desktop is offline',
        'Reset by the peer',
        'Connection closed',
        'Timeout',
        'Failed to connect to the remote device',
      }.contains(text);
}

int? unattendedRetryDelay(Duration elapsed, int previousDelay) {
  if (elapsed >= unattendedRetryWindow) return null;
  final remaining = (unattendedRetryWindow - elapsed).inSeconds;
  if (remaining <= 0) return null;
  return previousDelay.clamp(1, 5).clamp(1, remaining).toInt();
}
