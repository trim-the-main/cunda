import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common.dart';

String gitRevShaToHex(GitRevSha sha) {
  final sb = StringBuffer();
  for (final byte in sha.field0) {
    sb.write(byte.toRadixString(16).padLeft(2, '0'));
  }
  return sb.toString();
}

String firmwareVersionText(DeviceId deviceId) {
  final hash = deviceId.gitHash;
  if (hash != null) {
    return "${deviceId.firmwareVersion} (${gitRevShaToHex(hash).substring(0, 8)})";
  }
  return deviceId.firmwareVersion;
}
