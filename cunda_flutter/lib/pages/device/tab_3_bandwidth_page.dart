import 'package:cunda_flutter/pages/device/device_view_model.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

// ignore: unused_element
final _log = Logger('BandwidthTabPage');

class BandwidthTabPage extends ConsumerStatefulWidget {
  const BandwidthTabPage({super.key, required this.device});

  final BluetoothDevice device;

  @override
  ConsumerState<BandwidthTabPage> createState() => _BandwidthTabPageState();
}

class _BandwidthTabPageState extends ConsumerState<BandwidthTabPage> {
  bool _isUpstreamRunning = false;
  bool _isDownstreamRunning = false;

  @override
  Widget build(BuildContext context) {
    // Only watch the provider when running. When not running, the provider
    // loses all watchers — downstream's onCancel calls stopTestTopicBandwidth,
    // upstream's auto-dispose stops the testBandwidth loop.
    final upstreamThroughput = _isUpstreamRunning
        ? ref.watch(upstreamBandwidthProvider(widget.device)).value
        : null;

    final downstreamThroughput = _isDownstreamRunning
        ? ref.watch(downstreamBandwidthProvider(widget.device)).value
        : null;

    final sections = [
      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: _BandwidthSection(
            title: 'UPSTREAM',
            subtitle: 'Flutter → Device',
            isRunning: _isUpstreamRunning,
            throughput: upstreamThroughput ?? '0.00 KB/s',
            onToggle: () {
              setState(() {
                _isUpstreamRunning = !_isUpstreamRunning;
              });
            },
          ),
        ),
      ),
      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: _BandwidthSection(
            title: 'DOWNSTREAM',
            subtitle: 'Device → Flutter',
            isRunning: _isDownstreamRunning,
            throughput: downstreamThroughput ?? '0.00 KB/s',
            onToggle: () {
              setState(() {
                _isDownstreamRunning = !_isDownstreamRunning;
              });
            },
          ),
        ),
      ),
    ];

    return ListView.separated(
      padding: const EdgeInsets.all(20),
      itemCount: sections.length,
      separatorBuilder: (BuildContext context, int index) =>
          const SizedBox(height: 20),
      itemBuilder: (BuildContext context, int index) => sections[index],
    );
  }
}

class _BandwidthSection extends StatelessWidget {
  const _BandwidthSection({
    required this.title,
    required this.subtitle,
    required this.isRunning,
    required this.throughput,
    required this.onToggle,
  });

  final String title;
  final String subtitle;
  final bool isRunning;
  final String throughput;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: Theme.of(context)
              .textTheme
              .titleMedium
              ?.copyWith(fontWeight: FontWeight.bold),
        ),
        Text(
          subtitle,
          style: Theme.of(context).textTheme.bodySmall,
        ),
        const SizedBox(height: 12),
        Center(
          child: Text(
            throughput,
            style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                  fontFamily: 'monospace',
                  fontWeight: FontWeight.w500,
                ),
          ),
        ),
        const SizedBox(height: 12),
        Center(
          child: ElevatedButton(
            onPressed: onToggle,
            child: Text(isRunning ? 'Stop' : 'Start'),
          ),
        ),
        const SizedBox(height: 12),
      ],
    );
  }
}
