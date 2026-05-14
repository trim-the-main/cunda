import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/lib.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/endpoints.dart';
import 'package:cunda_flutter/pages/device/device_view_model.dart';
import 'package:cunda_flutter/providers/package_registry_provider.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/services/mayna/mayna_types.dart';
import 'package:cunda_flutter/services/mayna/package_registry.dart';
import 'package:cunda_flutter/services/mayna/version.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final _log = Logger('OtaSection');

class OtaSection extends ConsumerStatefulWidget {
  const OtaSection({super.key, required this.device});

  final BluetoothDevice device;

  @override
  ConsumerState<OtaSection> createState() => _OtaSectionState();
}

class _OtaSectionState extends ConsumerState<OtaSection> {
  bool _importing = false;
  String? _importResult;
  String? _importError;

  Future<void> _importPackage() async {
    final result = await FilePicker.platform.pickFiles();
    if (result == null) return;

    setState(() {
      _importing = true;
      _importResult = null;
      _importError = null;
    });

    try {
      final registry = await ref.read(packageRegistryProvider.future);
      final manifest = await registry.import(File(result.files.single.path!));
      if (mounted) {
        setState(() {
          _importing = false;
          _importResult =
              'Imported ${manifest.deviceType} v${manifest.firmwareVersion}';
        });
      }
    } catch (e) {
      _log.warning('Failed to import .mayna package: $e');
      if (mounted) {
        setState(() {
          _importing = false;
          _importError = 'Import failed: $e';
        });
      }
    }
  }

  void _startUpdate(File firmwareBinary, String version) {
    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (context) => ProgressDialog(
        device: widget.device,
        file: firmwareBinary,
        version: version,
      ),
    );
  }

  void _factoryResetAreYouSureDialog() {
    showDialog(
      context: context,
      builder: (BuildContext context) {
        return AlertDialog(
          title: const Text('Confirm Action'),
          content: const Text(
            'Are you sure you want to proceed with a factory reset?',
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('Cancel'),
            ),
            TextButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Confirm', style: TextStyle(color: Colors.red)),
            ),
          ],
        );
      },
    ).then((value) {
      if (value == true) {
        ref
            .read(endpointDispatcherProvider(widget.device))
            .when(
              data: (eDispatcher) async {
                await eDispatcher.factoryReset(req: const NoArg());
                if (mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text("Factory reset command sent successfully."),
                    ),
                  );
                }
              },
              error: (Object error, StackTrace stackTrace) {
                _log.warning("Error during factory reset: $error");
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text("Failed to factory reset the device."),
                  ),
                );
              },
              loading: () {
                _log.warning("eDispatcher is still loading...");
              },
            );
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return ref
        .watch(systemSettingsProvider(widget.device))
        .when(
          data: (settings) => _buildContent(context),
          error: (Object error, StackTrace stackTrace) =>
              Text("Error getting system settings"),
          loading: () => Center(child: CircularProgressIndicator()),
        );
  }

  Widget _buildContent(BuildContext context) {
    final deviceIdAsync = ref.watch(deviceIdProvider(widget.device));
    final registryAsync = ref.watch(packageRegistryProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Import section
        Text(
          "Package Import".toUpperCase(),
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
        ),
        SizedBox(height: 12),
        if (_importResult != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(_importResult!),
          ),
        if (_importError != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(_importError!, style: TextStyle(color: Colors.red)),
          ),
        ElevatedButton(
          onPressed: _importing ? null : _importPackage,
          child: _importing
              ? SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text("Import .mayna Package"),
        ),

        SizedBox(height: 32),

        // Update section
        Text(
          "Firmware Update".toUpperCase(),
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
        ),
        SizedBox(height: 12),
        _buildUpdateSection(deviceIdAsync, registryAsync),

        SizedBox(height: 32),

        // Factory reset section
        Text(
          "Factory Reset".toUpperCase(),
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
        ),
        SizedBox(height: 12),
        ElevatedButton(
          onPressed: _factoryResetAreYouSureDialog,
          child: Text("Reset"),
        ),
      ],
    );
  }

  Widget _buildUpdateSection(
    AsyncValue<DeviceId> deviceIdAsync,
    AsyncValue<PackageRegistry> registryAsync,
  ) {
    if (deviceIdAsync is AsyncLoading || registryAsync is AsyncLoading) {
      return Center(child: CircularProgressIndicator());
    }
    if (deviceIdAsync is AsyncError) {
      return Text("Error getting device info");
    }
    if (registryAsync is AsyncError) {
      return Text("Error loading package registry");
    }

    final deviceId = deviceIdAsync.value!;
    final registry = registryAsync.value!;

    final candidate = registry.findLatestCompatible(
      deviceType: deviceId.deviceType,
      currentFirmwareVersion: deviceId.firmwareVersion,
      supportedProtocols: {deviceId.protocolVersion},
    );

    if (candidate == null ||
        !isValidUpdate(deviceId.firmwareVersion, candidate.firmwareVersion)) {
      return Text("No update available");
    }

    final firmwareBinary = registry.componentPath(
      candidate.deviceType,
      candidate.firmwareVersion,
      ComponentType.firmware,
    );

    if (firmwareBinary == null) {
      return Text(
        "Update v${candidate.firmwareVersion} available "
        "but firmware binary missing from package",
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text("Update available: v${candidate.firmwareVersion}"),
        SizedBox(height: 8),
        ElevatedButton(
          onPressed: () =>
              _startUpdate(firmwareBinary, candidate.firmwareVersion),
          child: Text("Update to v${candidate.firmwareVersion}"),
        ),
      ],
    );
  }
}

class ProgressDialog extends ConsumerStatefulWidget {
  final BluetoothDevice device;
  final File file;
  final String version;

  const ProgressDialog({
    super.key,
    required this.device,
    required this.file,
    required this.version,
  });

  @override
  ConsumerState<ProgressDialog> createState() => _ProgressDialogState();
}

class _ProgressDialogState extends ConsumerState<ProgressDialog> {
  double _progress = 0.0;
  bool _isCancelled = false;
  bool _hasError = false;

  @override
  void initState() {
    super.initState();
    _startTransfer();
  }

  Future<void> _startTransfer() async {
    int fileSize = widget.file.lengthSync();

    final stream = widget.file.openRead();
    final hash = await sha256.bind(stream).first;
    U8Array32 hashArray = U8Array32(Uint8List.fromList(hash.bytes));

    OtaMData otaMData = OtaMData(
      size: fileSize,
      hashSha256: hashArray,
      version: widget.version,
    );

    final int chunkSize = 4096;
    int offset = 0;

    ref
        .read(endpointDispatcherProvider(widget.device))
        .when(
          data: (eDispatcher) async {
            setState(() {
              _hasError = false;
            });
            if (await eDispatcher.prepareOta(req: otaMData) !=
                OtaResult.transferReady) {
              _log.warning("Failed to prepare OTA");
              setState(() {
                _hasError = true;
              });
              return;
            }
            _log.fine("OTA preparation successful, starting transfer");
            while (offset < fileSize) {
              _log.fine("Transferring chunk at offset $offset");
              if (_isCancelled || !mounted) return;

              int bytesToRead = (offset + chunkSize > fileSize)
                  ? fileSize - offset
                  : chunkSize;

              List<int> chunkData = widget.file.readAsBytesSync().sublist(
                offset,
                offset + bytesToRead,
              );

              OtaBytes otaBytes = OtaBytes(
                offset: offset,
                data: Uint8List.fromList(chunkData),
              );

              OtaResult otaResult = await eDispatcher.transferOtaBytes(
                req: otaBytes,
              );
              if (otaResult != OtaResult.transferReady) {
                _log.warning(
                  "Failed to transfer OTA bytes with error: $otaResult",
                );
                setState(() {
                  _hasError = true;
                  _progress = 0.0;
                });
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
            await eDispatcher.finalizeOta(req: const NoArg());
          },
          error: (Object error, StackTrace stackTrace) {
            _log.warning("Error during OTA transfer: $error");
            setState(() {
              _hasError = true;
            });
          },
          loading: () {
            _log.warning("eDispatcher is still loading...");
            setState(() {
              _progress = 0.0;
              _hasError = false;
            });
          },
        );
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
