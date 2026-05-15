import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/defmt_log_translation.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:cunda_flutter/pages/device/device_view_model.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final _log = Logger('LogsPage');

class LogsPage extends ConsumerStatefulWidget {
  final BluetoothDevice device;

  const LogsPage({super.key, required this.device});

  @override
  ConsumerState<LogsPage> createState() => _LogsPageState();
}

class _LogsPageState extends ConsumerState<LogsPage> {
  final List<String> _logLines = [];
  final ScrollController _scrollController = ScrollController();
  ProviderSubscription? _logSubscription;

  @override
  void dispose() {
    _logSubscription?.close();
    _scrollController.dispose();
    super.dispose();
  }

  void _startListening(LogDecoder decoder) {
    _logSubscription = ref.listenManual(deviceLogsProvider(widget.device), (
      previous,
      next,
    ) {
      next.whenData((bytes) {
        try {
          final entries = decoder.decodeLog(bytes: bytes);
          setState(() {
            for (final entry in entries) {
              _logLines.add(_formatEntry(entry));
            }
          });
          _scrollToBottom();
        } catch (e) {
          _log.warning('Failed to decode log: $e');
        }
      });
    });
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
    final decoderAsync = ref.watch(
      initializedLogDecoderProvider(widget.device),
    );

    return Scaffold(
      appBar: AppBar(title: const Text('Device Logs')),
      body: decoderAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, st) => Center(child: Text('Error: $e')),
        data: (decoder) {
          if (decoder == null) {
            return const Center(
              child: Padding(
                padding: EdgeInsets.all(24),
                child: Text(
                  'Install a .mayna package for this firmware version '
                  'to view decoded logs.',
                  textAlign: TextAlign.center,
                ),
              ),
            );
          }
          if (_logSubscription == null) {
            _startListening(decoder);
          }
          return ListView.builder(
            controller: _scrollController,
            itemCount: _logLines.length,
            itemBuilder: (context, index) => Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 1),
              child: Text(
                _logLines[index],
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              ),
            ),
          );
        },
      ),
    );
  }
}
