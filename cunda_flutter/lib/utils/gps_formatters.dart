String formatLatitude(int? latE7) {
  return _formatCoordinate(latE7, 'N', 'S');
}

String formatLongitude(int? lonE7) {
  return _formatCoordinate(lonE7, 'E', 'W');
}

String _formatCoordinate(int? valueE7, String positiveDir, String negativeDir) {
  if (valueE7 == null) {
    return "-";
  }
  final direction = valueE7 >= 0 ? positiveDir : negativeDir;
  final absE7 = valueE7.abs();

  var degrees = absE7 ~/ 10000000;
  var minutes = (absE7 % 10000000) * 60 / 10000000;

  // Handle rounding up to 60.000' by carrying into degrees
  var minutesRounded = double.parse(minutes.toStringAsFixed(3));
  if (minutesRounded >= 60) {
    degrees += 1;
    minutesRounded -= 60;
  }

  final minutesStr = minutesRounded.toStringAsFixed(3).padLeft(6, '0');

  return "$degrees°$minutesStr' $direction";
}

String formatSog(int? sogMilliMeterPerSecond) {
  if (sogMilliMeterPerSecond == null) {
    return "-";
  }

  const double knots = 1852000 / 3600;
  const double knotsConversionFactor = 1.0 / knots;
  final double sogKnots = sogMilliMeterPerSecond * knotsConversionFactor;
  final sogStr = sogKnots.toStringAsFixed(1);
  return sogStr;
}

String formatCog(int? cogE5) {
  if (cogE5 == null) {
    return "-";
  }

  final double cog = cogE5 / 1e5;
  final cogStr = cog.toStringAsFixed(0);
  return cogStr;
}

String formatUtcTime(DateTime? dt) {
  if (dt == null) {
    return "-";
  }
  return dt.toString();
}
