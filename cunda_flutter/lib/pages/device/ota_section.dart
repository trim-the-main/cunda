import 'dart:io';
import 'dart:typed_data';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/lib.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/types.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/package_registry_provider.dart';
import 'package:cunda_flutter/providers/rpc/client.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/services/mayna/mayna_types.dart';
import 'package:cunda_flutter/services/mayna/package_registry.dart';
import 'package:cunda_flutter/services/mayna/version.dart';
import 'package:cunda_flutter/utils/confirm_dialog.dart';
import 'package:cunda_flutter/utils/rust_type_helpers.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final _log = Logger('OtaSection');

Uint8List _hexToBytes(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (int i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

OtaMData _otaMDataFromManifest(MaynaPackageManifest manifest) {
  final firmwareComponent = manifest.components[ComponentType.firmware]!;
  return OtaMData(
    size: firmwareComponent.size,
    hashSha256: U8Array32(_hexToBytes(firmwareComponent.sha256)),
    version: manifest.firmwareVersion,
  );
}

class OtaSection extends ConsumerWidget {
  const OtaSection({super.key, required this.device});

  final BluetoothDevice device;

  void _startUpdate(
    BuildContext context,
    CundaSysE dispatcher,
    OtaMData otaMData,
    File firmwareBinary,
  ) {
    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (context) => FirmwareFlashDialog(
        otaMData: otaMData,
        file: firmwareBinary,
        dispatcher: dispatcher,
      ),
    );
  }

  Future<void> _factoryReset(BuildContext context, CundaSysE dispatcher) async {
    final confirmed = await showConfirmDialog(
      context: context,
      title: 'Confirm Action',
      body: 'Are you sure you want to proceed with a factory reset?',
      destructive: true,
    );
    if (!confirmed || !context.mounted) return;

    try {
      await dispatcher.factoryReset(req: const NoArg());
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text("Factory reset command sent successfully."),
          ),
        );
      }
    } catch (e) {
      _log.warning("Error during factory reset: $e");
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text("Failed to factory reset the device.")),
        );
      }
    }
  }

  Future<void> _forceFirmwareUpdate(
    BuildContext context,
    CundaSysE dispatcher,
    DeviceId deviceId,
    PackageRegistry registry,
  ) async {
    final confirmed = await showConfirmDialog(
      context: context,
      title: 'Force firmware update?',
      body:
          'This bypasses safety checks and can downgrade or '
          'brick the device. Continue?',
      confirmLabel: 'Continue',
      destructive: true,
    );
    if (!confirmed || !context.mounted) return;

    final result = await FilePicker.platform.pickFiles();
    if (result == null || !context.mounted) return;

    MaynaPackageManifest manifest;
    try {
      manifest = await registry.import(File(result.files.single.path!));
    } catch (e) {
      _log.warning('Failed to import .mayna package: $e');
      if (context.mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Import failed: $e')));
      }
      return;
    }
    if (!context.mounted) return;

    if (manifest.deviceType != deviceId.deviceType) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            'Package is for ${manifest.deviceType}, '
            'device is ${deviceId.deviceType}',
          ),
        ),
      );
      return;
    }

    final firmwareBinary = registry.componentPath(
      manifest.deviceType,
      manifest.firmwareVersion,
      ComponentType.firmware,
    );
    if (firmwareBinary == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Firmware binary missing from package')),
      );
      return;
    }

    _startUpdate(
      context,
      dispatcher,
      _otaMDataFromManifest(manifest),
      firmwareBinary,
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dispatcherAsync = ref.watch(sysEndpointsProvider(device));
    final deviceIdAsync = ref.watch(deviceIdProvider(device));
    final registryAsync = ref.watch(packageRegistryProvider);

    if (dispatcherAsync is AsyncLoading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (dispatcherAsync is AsyncError) {
      return const Center(child: Text('Device disconnected'));
    }
    if (deviceIdAsync is AsyncLoading || registryAsync is AsyncLoading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (deviceIdAsync is AsyncError) {
      return const Text('Error loading device info');
    }
    if (registryAsync is AsyncError) {
      return const Text('Error loading package registry');
    }

    final dispatcher = dispatcherAsync.value!;
    final deviceId = deviceIdAsync.value!;
    final registry = registryAsync.value!;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _UpdateBannerCard(
          deviceId: deviceId,
          registry: registry,
          onUpdate: (manifest, file) => _startUpdate(
            context,
            dispatcher,
            _otaMDataFromManifest(manifest),
            file,
          ),
        ),
        const SizedBox(height: 16),
        _CurrentFirmwareCard(deviceId: deviceId, registry: registry),
        const SizedBox(height: 16),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Factory Reset',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const SizedBox(height: 12),
                ElevatedButton(
                  onPressed: () => _factoryReset(context, dispatcher),
                  child: const Text('Reset'),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 24),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              OutlinedButton(
                onPressed: () => _forceFirmwareUpdate(
                  context,
                  dispatcher,
                  deviceId,
                  registry,
                ),
                style: OutlinedButton.styleFrom(foregroundColor: Colors.red),
                child: const Text('Force firmware update'),
              ),
              const SizedBox(height: 6),
              Text(
                'Picks a .mayna file and flashes it without version checks.',
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: Colors.grey),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _UpdateBannerCard extends StatelessWidget {
  const _UpdateBannerCard({
    required this.deviceId,
    required this.registry,
    required this.onUpdate,
  });

  final DeviceId deviceId;
  final PackageRegistry registry;
  final void Function(MaynaPackageManifest manifest, File firmwareBinary)
  onUpdate;

  @override
  Widget build(BuildContext context) {
    final candidate = registry.findLatestCompatible(
      deviceType: deviceId.deviceType,
      currentFirmwareVersion: deviceId.firmwareVersion,
      supportedProtocols: {deviceId.protocolVersion},
    );

    final hasUpdate =
        candidate != null &&
        isValidUpdate(deviceId.firmwareVersion, candidate.firmwareVersion);

    if (!hasUpdate) {
      return Card(
        color: Colors.green.shade50,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              const Icon(Icons.check_circle, color: Colors.green),
              const SizedBox(width: 12),
              Text(
                'Up to date',
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
              ),
            ],
          ),
        ),
      );
    }

    final firmwareBinary = registry.componentPath(
      candidate.deviceType,
      candidate.firmwareVersion,
      ComponentType.firmware,
    );
    final changelogFile = registry.componentPath(
      candidate.deviceType,
      candidate.firmwareVersion,
      ComponentType.changelog,
    );

    return Card(
      color: Colors.blue.shade50,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.system_update, color: Colors.blue),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'New version available: v${candidate.firmwareVersion}',
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text('Published: ${candidate.publishDate}'),
            if (changelogFile != null) ...[
              const SizedBox(height: 8),
              _ChangelogTile(file: changelogFile),
            ],
            const SizedBox(height: 12),
            if (firmwareBinary == null)
              const Text(
                'Firmware binary missing from package',
                style: TextStyle(color: Colors.red),
              )
            else
              ElevatedButton(
                onPressed: () => onUpdate(candidate, firmwareBinary),
                child: Text('Update to v${candidate.firmwareVersion}'),
              ),
          ],
        ),
      ),
    );
  }
}

class _ChangelogTile extends StatelessWidget {
  const _ChangelogTile({required this.file});

  final File file;

  @override
  Widget build(BuildContext context) {
    return ExpansionTile(
      tilePadding: EdgeInsets.zero,
      childrenPadding: const EdgeInsets.only(bottom: 8),
      title: const Text('Changelog'),
      children: [
        FutureBuilder<String>(
          future: file.readAsString(),
          builder: (context, snapshot) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const Padding(
                padding: EdgeInsets.all(8),
                child: SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              );
            }
            if (snapshot.hasError) {
              return Text(
                'Failed to read changelog: ${snapshot.error}',
                style: const TextStyle(color: Colors.red),
              );
            }
            return Align(
              alignment: Alignment.centerLeft,
              child: Text(
                snapshot.data ?? '',
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              ),
            );
          },
        ),
      ],
    );
  }
}

class _CurrentFirmwareCard extends StatelessWidget {
  const _CurrentFirmwareCard({required this.deviceId, required this.registry});

  final DeviceId deviceId;
  final PackageRegistry registry;

  @override
  Widget build(BuildContext context) {
    final installed = registry.find(
      deviceId.deviceType,
      deviceId.firmwareVersion,
    );
    final publishDate = installed?.publishDate;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Current Firmware',
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 12),
            _kv(context, 'Version', firmwareVersionText(deviceId)),
            _kv(context, 'Device type', deviceId.deviceType),
            _kv(context, 'Protocol', deviceId.protocolVersion.toString()),
            if (publishDate != null) _kv(context, 'Published', publishDate),
          ],
        ),
      ),
    );
  }

  Widget _kv(BuildContext context, String key, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 110,
            child: Text(key, style: TextStyle(color: Colors.grey.shade700)),
          ),
          Expanded(child: Text(value)),
        ],
      ),
    );
  }
}

class FirmwareFlashDialog extends StatefulWidget {
  final OtaMData otaMData;
  final File file;
  final CundaSysE dispatcher;

  const FirmwareFlashDialog({
    super.key,
    required this.otaMData,
    required this.file,
    required this.dispatcher,
  });

  @override
  State<FirmwareFlashDialog> createState() => _FirmwareFlashDialogState();
}

class _FirmwareFlashDialogState extends State<FirmwareFlashDialog> {
  double _progress = 0.0;
  bool _isCancelled = false;
  bool _hasError = false;

  @override
  void initState() {
    super.initState();
    _startTransfer();
  }

  Future<void> _startTransfer() async {
    final int fileSize = widget.otaMData.size;
    final int chunkSize = 4096;
    int offset = 0;

    try {
      final Uint8List bytes = await widget.file.readAsBytes();

      if (await widget.dispatcher.prepareOta(req: widget.otaMData) !=
          OtaResult.transferReady) {
        _log.warning("Failed to prepare OTA");
        if (mounted) setState(() => _hasError = true);
        return;
      }
      _log.fine("OTA preparation successful, starting transfer");

      while (offset < fileSize) {
        _log.fine("Transferring chunk at offset $offset");
        if (_isCancelled || !mounted) return;

        final int bytesToRead = (offset + chunkSize > fileSize)
            ? fileSize - offset
            : chunkSize;

        final OtaBytes otaBytes = OtaBytes(
          offset: offset,
          data: Uint8List.sublistView(bytes, offset, offset + bytesToRead),
        );

        final OtaResult otaResult = await widget.dispatcher.transferOtaBytes(
          req: otaBytes,
        );
        if (otaResult != OtaResult.transferReady) {
          _log.warning("Failed to transfer OTA bytes with error: $otaResult");
          if (mounted) {
            setState(() {
              _hasError = true;
              _progress = 0.0;
            });
          }
          return;
        }
        offset += bytesToRead;

        if (mounted) {
          setState(() {
            _progress = offset / fileSize;
          });
        }
      }
      if (_isCancelled || !mounted) return;
      await widget.dispatcher.finalizeOta(req: const NoArg());
    } catch (e) {
      _log.warning("Error during OTA transfer: $e");
      if (mounted) setState(() => _hasError = true);
    }
  }

  @override
  Widget build(BuildContext context) {
    Widget content;
    Widget button = TextButton(
      onPressed: () {
        _isCancelled = true;
        Navigator.of(context).pop();
      },
      child: const Text("Cancel", style: TextStyle(color: Colors.red)),
    );
    if (_hasError) {
      content = Text("Failed to update firmware. Please try again.");
      button = TextButton(
        onPressed: () {
          Navigator.of(context).pop();
        },
        child: const Text("Close", style: TextStyle(color: Colors.red)),
      );
    } else if (_progress == 0.0) {
      content = Text("Preparing the update...");
    } else if (_progress == 1.0) {
      content = Text("Firmware update complete! Device is restarting...");
      button = TextButton(
        onPressed: () {
          Navigator.of(context).pop();
        },
        child: const Text("Close", style: TextStyle(color: Colors.red)),
      );
    } else {
      content = Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          LinearProgressIndicator(value: _progress),
          const SizedBox(height: 15),
          Text("${(_progress * 100).toInt()}% Complete"),
        ],
      );
    }
    return AlertDialog(
      title: Text("Updating firmware"),
      content: content,
      actions: [button],
    );
  }
}
