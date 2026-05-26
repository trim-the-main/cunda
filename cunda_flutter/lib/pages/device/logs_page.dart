import 'dart:collection';
import 'dart:io';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/defmt_log_translation.dart';
import 'package:cunda_flutter/providers/package_registry_provider.dart';
import 'package:cunda_flutter/providers/rpc/device_logs.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final _log = Logger('LogsPage');

class LogsPage extends StatelessWidget {
  final BluetoothDevice device;

  const LogsPage({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Device Logs')),
      body: Column(
        children: [
          _LogTopicSwitch(device: device),
          Expanded(child: _LogListView(device: device)),
        ],
      ),
    );
  }
}

class _LogTopicSwitch extends ConsumerWidget {
  final BluetoothDevice device;

  const _LogTopicSwitch({required this.device});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final keepRunning = ref.watch(logTopicEnabledProvider(device));
    return SwitchListTile(
      title: const Text('Keep logs running in background'),
      value: keepRunning,
      onChanged: (_) =>
          ref.read(logTopicEnabledProvider(device).notifier).toggle(),
    );
  }
}

class _LogListView extends ConsumerStatefulWidget {
  final BluetoothDevice device;

  const _LogListView({required this.device});

  @override
  ConsumerState<_LogListView> createState() => _LogListViewState();
}

class _LogListViewState extends ConsumerState<_LogListView> {
  final ScrollController _scrollController = ScrollController();
  bool _importing = false;

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  Future<void> _importPackage() async {
    final result = await FilePicker.platform.pickFiles();
    if (result == null) return;

    setState(() => _importing = true);

    try {
      final registry = await ref.read(packageRegistryProvider.future);
      final manifest = await registry.import(File(result.files.single.path!));
      ref.invalidate(initializedLogDecoderProvider(widget.device));
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              'Imported ${manifest.deviceType} v${manifest.firmwareVersion}',
            ),
          ),
        );
      }
    } catch (e) {
      _log.warning('Failed to import .mayna package: $e');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Import failed: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _importing = false);
    }
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }
    });
  }

  String _formatEntry(DefmtLogEntry entry) {
    final level = entry.level?.name.toUpperCase() ?? '?';
    final ts = entry.timestamp.isNotEmpty ? entry.timestamp : '';
    if (ts.isNotEmpty) {
      return '[$ts] $level ${entry.msg}';
    }
    return '$level ${entry.msg}';
  }

  @override
  Widget build(BuildContext context) {
    final Queue<DefmtLogEntry>? logs = ref.watch(
      decodedDeviceLogsProvider(widget.device),
    );

    if (logs == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('Firmware version not recognized'),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _importing ? null : _importPackage,
              child: _importing
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Import .mayna Package'),
            ),
          ],
        ),
      );
    }

    _scrollToBottom();

    return ListView.builder(
      controller: _scrollController,
      itemCount: logs.length,
      itemBuilder: (context, index) => Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 1),
        child: Text(
          _formatEntry(logs.elementAt(index)),
          style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
        ),
      ),
    );
  }
}
