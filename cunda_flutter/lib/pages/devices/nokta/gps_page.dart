import 'dart:async';

import 'package:cunda_flutter/pages/devices/demo_esp32/tab_0_application_page.dart';
import 'package:cunda_flutter/providers/rpc/cunda_gps.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:cunda_flutter/utils/gps_formatters.dart';
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
          child: ParsedGpsSection(widget: widget),
        ),
      ),
      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: RawNmeaSection(device: widget.device),
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

  TableRow _rowHelper(
    BuildContext context,
    Icon icon,
    String title,
    Widget valueWidget,
  ) {
    return TableRow(
      children: [
        icon,
        Padding(
          padding: const EdgeInsets.all(12.0),
          child: Text(title, style: Theme.of(context).textTheme.titleMedium),
        ),
        Align(alignment: Alignment.centerRight, child: valueWidget),
      ],
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final gpsData = ref.watch(parsedGpsStreamProvider(widget.device));
    return gpsData.when(
      data: (data) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            "Gps Data".toUpperCase(),
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
          ),
          SizedBox(height: 12),
          Row(
            mainAxisSize: MainAxisSize.max,
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              _sogCard(context, data.sog),
              _cogCard(context, data.cog),
            ],
          ),
          Table(
            border: TableBorder(horizontalInside: BorderSide()),
            defaultVerticalAlignment: TableCellVerticalAlignment.middle,

            columnWidths: const <int, TableColumnWidth>{
              0: IntrinsicColumnWidth(),
              1: FixedColumnWidth(96),
              2: FlexColumnWidth(),
            },
            children: [
              _rowHelper(
                context,
                Icon(Icons.watch),
                "UTC",
                Text(
                  formatUtcTime(
                    data.utcDate?.isDisposed ?? true
                        ? null
                        : data.utcDateTime(),
                  ),
                ),
              ),

              _rowHelper(
                context,
                Icon(Icons.gps_fixed),
                "Lat",
                Text(formatLatitude(data.lat)),
              ),
              _rowHelper(
                context,
                Icon(Icons.gps_fixed),
                "Lon",
                Text(formatLongitude(data.lon)),
              ),
              _rowHelper(
                context,
                Icon(Icons.satellite_alt),
                "Sat. used",
                Text(data.numSatellitesUsed.toString()),
              ),
            ],
          ),
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
      skipLoadingOnRefresh: false,
      skipLoadingOnReload: false,
    );
  }

  Widget _squareCardHelper(
    BuildContext context,
    String displayValue,
    Icon icon,
    String title, {
    double? width,
    String? units,
  }) {
    return SizedBox(
      width: width ?? 130,
      child: Card(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  icon,
                  SizedBox(width: 4),
                  Text(title, style: Theme.of(context).textTheme.titleMedium),
                ],
              ),
              SizedBox(height: 4),
              Row(
                children: [
                  Text(
                    displayValue,
                    style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  if (units != null)
                    Text(
                      units,
                      style: Theme.of(context).textTheme.headlineMedium,
                    ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _sogCard(BuildContext context, int? sog) {
    const icon = Icon(Icons.speed);

    return _squareCardHelper(
      context,
      formatSog(sog),
      icon,
      "SOG",
      units: sog != null ? "kt" : "",
    );
  }

  Widget _cogCard(BuildContext context, int? sog) {
    const icon = Icon(Icons.compass_calibration_rounded);

    return _squareCardHelper(
      context,
      formatCog(sog),
      icon,
      "COG",
      units: sog != null ? "°" : "",
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
        _RawNmeaTopicSwitch(device: widget.device),
        SizedBox(height: 12),

        ConsoleLikeTextField(txtStream: _rawNmea.stream, rowCount: 10),
        SizedBox(height: 12),
      ],
    );
  }
}

class _RawNmeaTopicSwitch extends ConsumerWidget {
  final BluetoothDevice device;

  const _RawNmeaTopicSwitch({required this.device});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final keepRunning = ref.watch(rawNmeaTopicEnabledProvider(device));
    if (keepRunning.hasValue) {
      return SwitchListTile(
        title: const Text("RAW NMEA 0183"),
        value: keepRunning.value!,
        onChanged: (_) => ref
            .read(rawNmeaTopicEnabledProvider(device).notifier)
            .set(!keepRunning.value!),
      );
    } else {
      return CircularProgressIndicator();
    }
  }
}
