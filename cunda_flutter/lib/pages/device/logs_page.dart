import 'dart:collection';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/defmt_log_translation.dart';
import 'package:cunda_flutter/providers/rpc/device_logs.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

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

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
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
    final Queue<DefmtLogEntry> logs =
        ref.watch(decodedDeviceLogsProvider(widget.device));

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
