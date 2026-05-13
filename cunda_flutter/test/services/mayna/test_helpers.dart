import 'dart:convert';
import 'dart:io';

import 'package:archive/archive.dart';
import 'package:crypto/crypto.dart';

import 'package:cunda_flutter/services/mayna/mayna_types.dart';
import 'package:cunda_flutter/services/mayna/package_registry.dart';

/// Create a test .mayna file (ZIP with manifest.json + firmware.bin).
Future<File> createTestMaynaFile(
  Directory tempDir, {
  required String deviceType,
  required String firmwareVersion,
  int protocolVersion = 1,
  String minFirmwareVersion = '0.0.0',
  String publishDate = '2026-01-01',
}) async {
  final firmwareData = utf8.encode('fake firmware data for $firmwareVersion');
  final firmwareSha256 =
      sha256.convert(firmwareData).toString();

  final manifest = {
    'format_version': 1,
    'device_type': deviceType,
    'firmware_version': firmwareVersion,
    'protocol_version': protocolVersion,
    'min_firmware_version': minFirmwareVersion,
    'publish_date': publishDate,
    'components': {
      'firmware': {
        'file': 'firmware.bin',
        'size': firmwareData.length,
        'sha256': firmwareSha256,
      },
    },
  };

  final archive = Archive();
  final manifestBytes = utf8.encode(jsonEncode(manifest));
  archive.addFile(ArchiveFile('manifest.json', manifestBytes.length, manifestBytes));
  archive.addFile(ArchiveFile('firmware.bin', firmwareData.length, firmwareData));

  final zipData = ZipEncoder().encode(archive);
  final file = File(
      '${tempDir.path}/${deviceType}_$firmwareVersion.mayna');
  await file.writeAsBytes(zipData!);
  return file;
}

/// Convenience: create and import a test package.
Future<MaynaPackageManifest> importTestPackage(
  PackageRegistry registry,
  Directory tempDir, {
  required String deviceType,
  required String firmwareVersion,
  int protocolVersion = 1,
  String minFirmwareVersion = '0.0.0',
}) async {
  final file = await createTestMaynaFile(
    tempDir,
    deviceType: deviceType,
    firmwareVersion: firmwareVersion,
    protocolVersion: protocolVersion,
    minFirmwareVersion: minFirmwareVersion,
  );
  return registry.import(file);
}
