import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/lib.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/endpoints.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final log = Logger('OtaSection');

class OtaSection extends ConsumerStatefulWidget {
  const OtaSection({super.key, required this.device});

  final BluetoothDevice device;

  @override
  ConsumerState<OtaSection> createState() => _OtaSectionState();
}

class _OtaSectionState extends ConsumerState<OtaSection> {
  File? firmwareFile;

  @override
  void initState() {
    super.initState();
  }

  @override
  void dispose() {
    super.dispose();
  }

  void update() {
    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (context) =>
          ProgressDialog(device: widget.device, file: firmwareFile!),
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
            // The "Confirm" button
            TextButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Confirm', style: TextStyle(color: Colors.red)),
            ),
          ],
        );
      },
    ).then((value) {
      // Check the result of the dialog
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
                log.warning("Error during factory reset: $error");
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text("Failed to factory reset the device."),
                  ),
                );
              },
              loading: () {
                log.warning("eDispatcher is still loading...");
              },
            );
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    List<Widget> tiles = [
      Text(
        "Firmware Update".toUpperCase(),
        style: Theme.of(
          context,
        ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
      ),
      SizedBox(height: 20),
    ];

    if (firmwareFile != null) {
      tiles.addAll([
        Text("Selected file: ${firmwareFile!.path}"),
        Text("File size: ${firmwareFile!.lengthSync()} bytes"),
        SizedBox(height: 12),
      ]);
    }

    tiles.addAll([
      ElevatedButton(
        onPressed: () async {
          FilePickerResult? result = await FilePicker.platform.pickFiles();
          log.fine("File picker result: $result");
          if (result != null) {
            setState(() {
              firmwareFile = File(result.files.single.path!);
            });
          }
        },
        child: Text("Choose Firmware File"),
      ),
      SizedBox(height: 12),

      ElevatedButton(
        onPressed: firmwareFile != null ? update : null,
        child: Text("Update"),
      ),
      SizedBox(height: 40),
      Text(
        "Factory Reset".toUpperCase(),
        style: Theme.of(
          context,
        ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
      ),
      SizedBox(height: 20),
      ElevatedButton(
        onPressed: _factoryResetAreYouSureDialog,
        child: Text("Reset"),
      ),
    ]);
    return ref
        .watch(systemSettingsProvider(widget.device))
        .when(
          data: (settings) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: tiles,
            );
          },
          error: (Object error, StackTrace stackTrace) =>
              Text("Error getting system settings"),
          loading: () => Center(child: CircularProgressIndicator()),
        );
  }
}

class ProgressDialog extends ConsumerStatefulWidget {
  final BluetoothDevice device;

  final File file;

  const ProgressDialog({super.key, required this.device, required this.file});

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
    // 1. This runs exactly ONCE when the dialog appears
    _startTransfer();
  }

  Future<void> _startTransfer() async {
    int fileSize = widget.file.lengthSync();

    final stream = widget.file.openRead();

    // 2. Pass the stream to the sha256 hash function
    final hash = await sha256.bind(stream).first;
    U8Array32 hashArray = U8Array32(Uint8List.fromList(hash.bytes));

    OtaMData otaMData = OtaMData(
      size: fileSize,
      hashSha256: hashArray,
      version: "2.0.0",
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
              log.warning("Failed to prepare OTA");
              setState(() {
                _hasError = true;
              });
              return;
            }
            log.fine("OTA preparation successful, starting transfer");
            while (offset < fileSize) {
              log.fine("Transferring chunk at offset $offset");
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
                log.warning(
                  "Failed to transfer OTA bytes with error: $otaResult",
                );
                setState(() {
                  _hasError = true;
                  _progress = 0.0;
                });
                return;
              }
              offset += bytesToRead;

              // Update progress
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
            log.warning("Error during OTA transfer: $error");
            setState(() {
              _hasError = true;
            });
          },
          loading: () {
            log.warning("eDispatcher is still loading...");
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
