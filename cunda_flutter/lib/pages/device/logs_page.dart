import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/defmt_log_translation.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc/client.dart';
import 'package:cunda_flutter/pages/device/device_view_model.dart';
import 'package:cunda_flutter/providers/package_registry_provider.dart';
import 'package:cunda_flutter/services/mayna/mayna_types.dart';
import 'package:cunda_flutter/providers/rpc/client.dart';
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
  bool? _decoderAvailable;
  FlutterClient? _client;

  @override
  void initState() {
    super.initState();
    _initDecoder();
  }

  @override
  void dispose() {
    _logSubscription?.close();
    _scrollController.dispose();
    super.dispose();
  }

  Future<void> _initDecoder() async {
    final deviceId = await ref.read(deviceIdProvider(widget.device).future);
    final registry = await ref.read(packageRegistryProvider.future);

    final tableFile = registry.componentPath(
      deviceId.deviceType,
      deviceId.firmwareVersion,
      ComponentType.defmtTable,
    );
    final locFile = registry.componentPath(
      deviceId.deviceType,
      deviceId.firmwareVersion,
      ComponentType.defmtLocations,
    );

    if (tableFile == null) {
      _log.info(
        'No defmt table available for '
        '${deviceId.deviceType}/${deviceId.firmwareVersion}',
      );
      if (mounted) setState(() => _decoderAvailable = false);
      return;
    }

    final client = await ref.read(rpcClientProvider(widget.device).future);
    client.initLogDecoder(
      tableBytes: await tableFile.readAsBytes(),
      locBytes: locFile != null ? await locFile.readAsBytes() : [],
    );
    _log.info('Log decoder initialized');
    if (mounted) {
      setState(() {
        _decoderAvailable = true;
        _client = client;
      });
      _startListening();
    }
  }

  void _startListening() {
    _logSubscription = ref.listenManual(deviceLogsProvider(widget.device), (
      previous,
      next,
    ) {
      next.whenData((bytes) {
        if (_client == null) return;
        try {
          final entries = _client!.decodeLog(bytes: bytes);
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
    return Scaffold(
      appBar: AppBar(title: const Text('Device Logs')),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_decoderAvailable == null) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_decoderAvailable == false) {
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
  }
}
