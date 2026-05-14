const String developmentVersion = "99.99.999";

/// Compare two semver strings (major.minor.patch).
///
/// Returns negative if [a] < [b], zero if equal, positive if [a] > [b].
int compareVersions(String a, String b) {
  final partsA = a.split('.').map(int.parse).toList();
  final partsB = b.split('.').map(int.parse).toList();
  for (var i = 0; i < 3; i++) {
    final va = i < partsA.length ? partsA[i] : 0;
    final vb = i < partsB.length ? partsB[i] : 0;
    if (va != vb) return va.compareTo(vb);
  }
  return 0;
}

/// Compare the version strings and determine if we can accept the version or not
bool isValidUpdate(String currentVersion, String intendedVersion) {
  if (currentVersion == developmentVersion ||
      intendedVersion == developmentVersion) {
    return true;
  } else {
    return compareVersions(intendedVersion, currentVersion) > 0;
  }
}
