import 'dart:io';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/types.dart';
import 'package:cunda_flutter/pages/device/device_view_model.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/providers/rpc/client.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:cunda_flutter/utils/rust_type_helpers.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final _log = Logger('SystemTabPage');

class SystemTabPage extends StatelessWidget {
  const SystemTabPage({super.key, required this.device});

  final BluetoothDevice device;

  @override
  Widget build(BuildContext context) {
    _log.fine("Building status tab page");
    final List<Widget> statusTiles = [
      BluetoothConnectionStatusCard(device: device),
      DeviceHealth(device: device),
      StatusTableWidget(device: device),
      SysStatsWidget(device: device),
    ];

    return ListView.separated(
      padding: EdgeInsets.all(20),
      itemCount: statusTiles.length,
      separatorBuilder: (BuildContext context, int index) =>
          const SizedBox(height: 20),
      itemBuilder: (BuildContext context, int index) {
        return statusTiles[index];
      },
    );
  }
}

enum RpcConnectionStatus {
  rpcConnecting,
  rpcConnected,
  notRpcDevice;

  Icon get icon {
    return switch (this) {
      rpcConnecting => Icon(Icons.bluetooth_searching),
      rpcConnected => Icon(Icons.bluetooth_connected),
      notRpcDevice => Icon(Icons.bluetooth_disabled),
    };
  }
}

class BluetoothConnectionStatusCard extends ConsumerWidget {
  const BluetoothConnectionStatusCard({super.key, required this.device});

  final BluetoothDevice device;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final deviceConn = ref.watch(connectionManagerProvider(device));
    final sysD = ref.watch(sysEndpointsProvider(device));
    return sysD.when(
      data: (_) =>
          connectionCard(context, deviceConn, RpcConnectionStatus.rpcConnected),
      error: (err, s) {
        if (err is NotRpcDeviceException) {
          return connectionCard(
            context,
            deviceConn,
            RpcConnectionStatus.notRpcDevice,
          );
        } else {
          // ConnectionLost
          return connectionCard(
            context,
            deviceConn,
            RpcConnectionStatus.rpcConnecting,
          );
        }
      },
      loading: () => connectionCard(
        context,
        deviceConn,
        RpcConnectionStatus.rpcConnecting,
      ),
    );
  }

  Widget connectionCard(
    BuildContext context,
    ConnectionTransitionState connState,
    RpcConnectionStatus rpcState,
  ) {
    final titleStr = switch (connState) {
      ConnectionTransitionState.connecting => "Connecting",
      ConnectionTransitionState.disconnecting => "Disconnecting",
      ConnectionTransitionState.disconnected => "Disconnected",
      ConnectionTransitionState.connected => switch (rpcState) {
        RpcConnectionStatus.rpcConnecting => "RPC Connecting",
        RpcConnectionStatus.rpcConnected => "RPC Connected",
        RpcConnectionStatus.notRpcDevice => "Not RPC device",
      },
    };
    final subtitleStr = switch (connState) {
      ConnectionTransitionState.connecting =>
        "Establishing bluetooth connection...",
      ConnectionTransitionState.disconnecting => "Disconnecting",
      ConnectionTransitionState.disconnected => "Disconnected",
      ConnectionTransitionState.connected => switch (rpcState) {
        RpcConnectionStatus.rpcConnecting => "Establishing RPC channel...",
        RpcConnectionStatus.rpcConnected => "Handshake successful",
        RpcConnectionStatus.notRpcDevice =>
          "RPC service is not running on the Bluetooth device",
      },
    };
    return Card(
      child: Consumer(
        builder: (context, ref, child) {
          return ListTile(
            leading: rpcState.icon,
            title: Text(
              titleStr,
              style: TextStyle(fontWeight: FontWeight.bold),
            ),
            subtitle: Text(subtitleStr, style: TextStyle(fontSize: 12)),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(10.0),
            ),
            onLongPress: () {
              showMenu(
                context: context,
                position: RelativeRect.fromLTRB(0, 0, 0, 0),
                items: [
                  PopupMenuItem<String>(
                    value: 'disconnect',
                    child: Text('Disconnect'),
                  ),
                ],
              ).then((value) {
                if (value == 'disconnect') {
                  ref
                      .read(connectionManagerProvider(device).notifier)
                      .disconnect();
                  _log.fine('async disconnect from device');
                }
              });
            },
          );
        },
      ),
    );
  }
}

class SquareCard extends StatelessWidget {
  final Icon icon;
  final String title;
  final String displayValue;
  final String? units;
  final double? indicatorValue; // in range [0, 1.0]

  final double width;

  const SquareCard({
    super.key,
    required this.icon,
    required this.title,
    required this.displayValue,
    this.units,
    this.indicatorValue,
    this.width = 160,
  });

  Color _color() {
    if (indicatorValue == null) {
      return Colors.grey;
    }
    if (indicatorValue! < 0.25) {
      return Colors.red;
    } else if (indicatorValue! < 0.5) {
      return Colors.orange;
    } else {
      return Colors.green;
    }
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: width,
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
                      units!,
                      style: Theme.of(context).textTheme.headlineMedium,
                    ),
                ],
              ),

              // TODO fix the size
              SizedBox(height: 4),
              Container(
                // width: width,
                child: indicatorValue != null
                    ? LinearProgressIndicator(
                        value: indicatorValue,
                        color: _color(),
                      )
                    : Container(),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class DeviceHealth extends ConsumerWidget {
  final BluetoothDevice device;
  const DeviceHealth({super.key, required this.device});

  Widget _mtuCard() {
    const icon = Icon(Icons.settings_ethernet);
    return Consumer(
      builder: (context, ref, _) {
        final value = !Platform.isLinux
            ? ref.watch(mtuStreamProvider(device))
            : ref.watch(getMtuFromDeviceProvider(device));

        return value.when(
          data: (mtu) {
            return SquareCard(
              icon: icon,
              title: "MTU",
              displayValue: "$mtu",
              indicatorValue: mtu.toDouble() / 256,
            );
          },
          error: (error, stackTrace) =>
              SquareCard(icon: icon, title: "MTU", displayValue: "Error"),
          loading: () =>
              SquareCard(icon: icon, title: "MTU", displayValue: "Loading"),
        );
      },
    );
  }

  Widget _rssiCard() {
    const Map<SignalStrength, Icon> iconMap = {
      SignalStrength.poor: Icon(Icons.signal_cellular_alt_1_bar_rounded),
      SignalStrength.fair: Icon(Icons.signal_cellular_alt_1_bar_rounded),
      SignalStrength.good: Icon(Icons.signal_cellular_alt_2_bar_rounded),
      SignalStrength.excellent: Icon(Icons.signal_cellular_4_bar_rounded),
    };
    const Map<SignalStrength, double> indicatorValueMap = {
      SignalStrength.poor: 0.1,
      SignalStrength.fair: 0.3,
      SignalStrength.good: 0.6,
      SignalStrength.excellent: 0.8,
    };

    final ret = Consumer(
      builder: (context, ref, _) => ref
          .watch(rssiStreamProvider(device))
          .when(
            data: (rssiValue) {
              final signal = SignalStrength.fromRssiValue(rssiValue);
              return SquareCard(
                icon: iconMap[signal]!,
                title: "Signal",
                displayValue: "$rssiValue",
                units: "dBm",
                indicatorValue: indicatorValueMap[signal],
              );
            },
            error: (error, stackTrace) => SquareCard(
              icon: iconMap[SignalStrength.poor]!,
              title: "Signal",
              displayValue: "Error",
            ),
            loading: () => CircularProgressIndicator(),
          ),
    );
    return ret;
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          "Device Info".toUpperCase(),
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
        ),
        SizedBox(height: 12),
        Row(
          mainAxisSize: MainAxisSize.max,
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [_rssiCard(), _mtuCard()],
        ),
      ],
    );
  }
}

class SysStatsWidget extends ConsumerWidget {
  const SysStatsWidget({super.key, required this.device});
  final BluetoothDevice device;
  Widget _buildSysStatsCard(BuildContext context, SysStats stats) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(8.0),
        child: Table(
          border: TableBorder(horizontalInside: BorderSide()),
          defaultVerticalAlignment: TableCellVerticalAlignment.middle,

          columnWidths: const <int, TableColumnWidth>{
            0: IntrinsicColumnWidth(),
            1: FlexColumnWidth(),
            2: FixedColumnWidth(64),
          },
          children: [
            TableRow(
              children: [
                Icon(Icons.computer),
                Padding(
                  padding: const EdgeInsets.all(12.0),
                  child: Text(
                    "Cpu Usage(core0)",
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                Text("${stats.cpuUsage.core0.field0}%"),
              ],
            ),
            TableRow(
              children: [
                Icon(Icons.computer),
                Padding(
                  padding: const EdgeInsets.all(12.0),
                  child: Text(
                    "Cpu Usage(core1)",
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                Text("${stats.cpuUsage.core1.field0}%"),
              ],
            ),
            TableRow(
              children: [
                Icon(Icons.memory),
                Padding(
                  padding: const EdgeInsets.all(12.0),
                  child: Text(
                    "Memory Usage",
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                Text("${stats.memoryUsage.used} / ${stats.memoryUsage.total}"),
              ],
            ),
            TableRow(
              children: [
                Icon(Icons.timelapse),
                Padding(
                  padding: const EdgeInsets.all(12.0),
                  child: Text(
                    "Uptime",
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                Text("${stats.uptime} secs"),
              ],
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final table = ref
        .watch(systemStatsStreamProvider(device))
        .when(
          data: (stats) => _buildSysStatsCard(context, stats),
          error: (error, stackTrace) =>
              Center(child: Text("Error getting system stats")),
          loading: () => Center(child: CircularProgressIndicator()),
        );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          "System Stats".toUpperCase(),
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
        ),
        SizedBox(height: 12),
        table,
      ],
    );
  }
}

class StatusTableWidget extends StatelessWidget {
  final BluetoothDevice device;

  const StatusTableWidget({super.key, required this.device});

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
        valueWidget,
      ],
    );
  }

  TableRow _mtuRow(BuildContext context) {
    return _rowHelper(
      context,
      Icon(Icons.settings_ethernet),
      "Max. Tranmission Unit",
      Consumer(
        builder: (context, ref, child) {
          final value = !Platform.isLinux
              ? ref.watch(mtuStreamProvider(device))
              : ref.watch(getMtuFromDeviceProvider(device));
          return value.when(
            data: (data) => Text("$data bytes"),
            error: (error, stackTrace) => Text("Error"),
            loading: () => LinearProgressIndicator(),
          );
        },
      ),
    );
  }

  TableRow _rssiRow(BuildContext context) {
    return _rowHelper(
      context,
      Icon(Icons.signal_cellular_alt_1_bar_rounded),
      "Signal",
      Consumer(
        builder: (context, ref, child) => ref
            .watch(rssiStreamProvider(device))
            .when(
              data: (rssiValue) => Text("$rssiValue dBm"),
              error: (error, stackTrace) => Text("Error"),
              loading: () => LinearProgressIndicator(),
            ),
      ),
    );
  }

  TableRow _firmwareVersionRow(BuildContext context) {
    return _rowHelper(
      context,
      Icon(Icons.developer_board),
      "Firmware Version",
      Consumer(
        builder: (context, ref, child) => ref
            .watch(deviceIdProvider(device))
            .when(
              data: (deviceId) => Text(firmwareVersionText(deviceId)),
              error: (error, stackTrace) => Text("Error"),
              loading: () => LinearProgressIndicator(),
            ),
      ),
    );
  }

  TableRow _pingRow(BuildContext context) {
    return _rowHelper(
      context,
      Icon(Icons.network_ping),
      "Ping",
      Consumer(
        builder: (context, ref, child) => ref
            .watch(pingStreamProvider(device))
            .when(
              data: (duration) => Text("${duration.inMilliseconds} ms"),
              error: (error, stackTrace) => Text("Error"),
              loading: () => LinearProgressIndicator(),
            ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(8.0),
        child: Table(
          border: TableBorder(horizontalInside: BorderSide()),
          defaultVerticalAlignment: TableCellVerticalAlignment.middle,

          columnWidths: const <int, TableColumnWidth>{
            0: IntrinsicColumnWidth(),
            1: FlexColumnWidth(),
            2: FixedColumnWidth(64),
          },
          children: [
            _firmwareVersionRow(context),
            _rssiRow(context),
            _pingRow(context),
            _mtuRow(context),
          ],
        ),
      ),
    );
  }
}
