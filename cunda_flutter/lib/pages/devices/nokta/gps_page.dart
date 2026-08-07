import 'dart:async';

import 'package:cunda_flutter/pages/devices/demo_esp32/tab_0_application_page.dart';
import 'package:cunda_flutter/providers/rpc/cunda_gps.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final _log = Logger('NoktaGpsPage');

class GpsPage extends StatefulWidget {
  const GpsPage({super.key, required this.device});

  final BluetoothDevice device;

  @override
  State<GpsPage> createState() => _GpsPageState();
}

class _GpsPageState extends State<GpsPage> with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _log.fine("initState of ApplicationTabPage");
  }

  @override
  void dispose() {
    _log.fine("dispose of ApplicationTabPage");
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final List<Widget> applTiles = [
      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: RawNmeaSection(device: widget.device),
        ),
      ),

      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: ParsedGpsSection(widget: widget),
        ),
      ),
    ];

    return ListView.separated(
      padding: EdgeInsets.all(20),
      itemCount: applTiles.length,
      separatorBuilder: (BuildContext context, int index) =>
          const SizedBox(height: 20),
      itemBuilder: (BuildContext context, int index) {
        return applTiles[index];
      },
    );
  }
}

class ParsedGpsSection extends ConsumerWidget {
  const ParsedGpsSection({super.key, required this.widget});

  final GpsPage widget;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final gpsData = ref.watch(parsedGpsStreamProvider(widget.device));
    return gpsData.when(
      data: (data) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            "Gps data".toUpperCase(),
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
          ),
          SizedBox(height: 12),
          Text("Date time (UTC)"),
          Text(data.utcDateTime().toString()),
          SizedBox(height: 12),
          Text("Latitude"),
          Text(data.lat.toString()),
          SizedBox(height: 12),
          Text("Longitude"),
          Text(data.lon.toString()),
          SizedBox(height: 12),
          Text("SOG"),
          Text(data.sog.toString()),
          SizedBox(height: 12),
          Text("COG"),
          Text(data.cog.toString()),
          SizedBox(height: 12),
          Text("Number of Satelites"),
          Text(data.numSatellitesUsed.toString()),
        ],
      ),
      error: (Object error, StackTrace stackTrace) {
        _log.warning(error);
        return Text(
          "Error establising connection to ${widget.device.chosenName}",
        );
      },
      loading: () {
        return Center(child: CircularProgressIndicator());
      },
    );
  }
}

class RawNmeaSection extends ConsumerStatefulWidget {
  const RawNmeaSection({super.key, required this.device});

  final BluetoothDevice device;

  @override
  ConsumerState<RawNmeaSection> createState() => _RawNmeaSectionState();
}

class _RawNmeaSectionState extends ConsumerState<RawNmeaSection> {
  final StreamController<String> _rawNmea = StreamController<String>();

  @override
  void initState() {
    super.initState();
  }

  @override
  void dispose() {
    _log.fine("Disposing ParsedGpsSection stateful widget");
    _rawNmea.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    _log.fine("Listening to parsed gps stream");

    ref.listen(rawNmeaSentenceStreamProvider(widget.device), (prev, next) {
      next.whenData((value) {
        _rawNmea.sink.add(value.trim());
      });
    });
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          "Raw Nmea 0183".toUpperCase(),
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
        ),
        SizedBox(height: 12),
        ConsoleLikeTextField(txtStream: _rawNmea.stream, rowCount: 10),
        SizedBox(height: 12),
      ],
    );
  }
}
