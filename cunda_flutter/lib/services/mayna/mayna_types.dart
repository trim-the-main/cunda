/// A single component (file) within a .mayna archive.
class MaynaComponent {
  final String file;
  final int size;
  final String sha256;

  const MaynaComponent({
    required this.file,
    required this.size,
    required this.sha256,
  });

  factory MaynaComponent.fromJson(Map<String, dynamic> json) {
    return MaynaComponent(
      file: json['file'] as String,
      size: json['size'] as int,
      sha256: json['sha256'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
        'file': file,
        'size': size,
        'sha256': sha256,
      };
}

/// The manifest.json inside a .mayna archive.
class MaynaPackageManifest {
  final int formatVersion;
  final String deviceType;
  final String firmwareVersion;
  final int protocolVersion;
  final String minFirmwareVersion;
  final String publishDate;
  final Map<String, MaynaComponent> components;

  const MaynaPackageManifest({
    required this.formatVersion,
    required this.deviceType,
    required this.firmwareVersion,
    required this.protocolVersion,
    required this.minFirmwareVersion,
    required this.publishDate,
    required this.components,
  });

  factory MaynaPackageManifest.fromJson(Map<String, dynamic> json) {
    final componentsJson = json['components'] as Map<String, dynamic>;
    final components = componentsJson.map(
      (key, value) =>
          MapEntry(key, MaynaComponent.fromJson(value as Map<String, dynamic>)),
    );
    return MaynaPackageManifest(
      formatVersion: json['format_version'] as int,
      deviceType: json['device_type'] as String,
      firmwareVersion: json['firmware_version'] as String,
      protocolVersion: json['protocol_version'] as int,
      minFirmwareVersion: json['min_firmware_version'] as String,
      publishDate: json['publish_date'] as String,
      components: components,
    );
  }

  Map<String, dynamic> toJson() => {
        'format_version': formatVersion,
        'device_type': deviceType,
        'firmware_version': firmwareVersion,
        'protocol_version': protocolVersion,
        'min_firmware_version': minFirmwareVersion,
        'publish_date': publishDate,
        'components':
            components.map((key, value) => MapEntry(key, value.toJson())),
      };
}

/// Entry from the remote manifest (not yet downloaded).
class RemotePackageInfo {
  final String deviceType;
  final String firmwareVersion;
  final int protocolVersion;
  final String minFirmwareVersion;
  final String publishDate;
  final Uri packageUrl;
  final int packageSize;
  final String packageSha256;

  const RemotePackageInfo({
    required this.deviceType,
    required this.firmwareVersion,
    required this.protocolVersion,
    required this.minFirmwareVersion,
    required this.publishDate,
    required this.packageUrl,
    required this.packageSize,
    required this.packageSha256,
  });

  factory RemotePackageInfo.fromJson(Map<String, dynamic> json) {
    return RemotePackageInfo(
      deviceType: json['device_type'] as String,
      firmwareVersion: json['firmware_version'] as String,
      protocolVersion: json['protocol_version'] as int,
      minFirmwareVersion: json['min_firmware_version'] as String,
      publishDate: json['publish_date'] as String,
      packageUrl: Uri.parse(json['package_url'] as String),
      packageSize: json['package_size'] as int,
      packageSha256: json['package_sha256'] as String,
    );
  }
}

/// Remote manifest (fetched from static file host).
class RemoteManifest {
  final int manifestVersion;
  final int sequenceNumber;
  final List<RemotePackageInfo> packages;

  const RemoteManifest({
    required this.manifestVersion,
    required this.sequenceNumber,
    required this.packages,
  });

  factory RemoteManifest.fromJson(Map<String, dynamic> json) {
    final packagesJson = json['packages'] as List<dynamic>;
    return RemoteManifest(
      manifestVersion: json['manifest_version'] as int,
      sequenceNumber: json['sequence_number'] as int,
      packages: packagesJson
          .map((e) => RemotePackageInfo.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }
}

/// A known device that the app tracks.
class KnownDevice {
  final int serialNumber;
  final String deviceType;
  final String firmwareVersion;
  final DateTime lastSeen;

  const KnownDevice({
    required this.serialNumber,
    required this.deviceType,
    required this.firmwareVersion,
    required this.lastSeen,
  });

  factory KnownDevice.fromJson(Map<String, dynamic> json) {
    return KnownDevice(
      serialNumber: json['serial_number'] as int,
      deviceType: json['device_type'] as String,
      firmwareVersion: json['firmware_version'] as String,
      lastSeen: DateTime.parse(json['last_seen'] as String),
    );
  }

  bool matches(String deviceType, int serialNumber) =>
      this.deviceType == deviceType && this.serialNumber == serialNumber;

  Map<String, dynamic> toJson() => {
        'serial_number': serialNumber,
        'device_type': deviceType,
        'firmware_version': firmwareVersion,
        'last_seen': lastSeen.toIso8601String(),
      };
}
