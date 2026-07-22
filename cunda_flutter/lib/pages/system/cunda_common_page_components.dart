import 'package:cunda_flutter/pages/system/logs_page.dart';
import 'package:cunda_flutter/pages/system/ota_page.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

// ignore: unused_element
final _log = Logger('CundaCommonPageComponents');

typedef PageBuilder =
    Widget Function(BuildContext context, BluetoothDevice device);

abstract interface class DotMenuAction {
  String get label;
  PageBuilder get pageBuilder;
}

enum CundaCommonAction implements DotMenuAction {
  logs(label: 'Logs', pageBuilder: _logsPageBuilder),
  ota(label: 'Firmware Update', pageBuilder: _otaPageBuilder);

  const CundaCommonAction({required this.label, required this.pageBuilder});

  @override
  final String label;
  @override
  final PageBuilder pageBuilder;

  static Widget _otaPageBuilder(BuildContext _, BluetoothDevice device) =>
      OtaPage(device: device);

  static Widget _logsPageBuilder(BuildContext _, BluetoothDevice device) =>
      LogsPage(device: device);
}

class CundaDeviceAppBar extends StatelessWidget implements PreferredSizeWidget {
  final BluetoothDevice device;
  final List<DotMenuAction> dotMenuActions;
  final PreferredSizeWidget? tabBar;

  const CundaDeviceAppBar({
    super.key,
    required this.device,
    required this.dotMenuActions,
    this.tabBar,
  });

  @override
  Widget build(BuildContext context) {
    return AppBar(
      title: _connectionPageTitle(),
      centerTitle: true,
      actions: [
        PopupMenuButton<PageBuilder>(
          onSelected: (pageBuilder) => Navigator.push(
            context,
            MaterialPageRoute(
              builder: (context) => pageBuilder(context, device),
            ),
          ),
          itemBuilder: (BuildContext context) => dotMenuActions
              .map(
                (action) => PopupMenuItem(
                  value: action.pageBuilder,
                  child: Text(action.label),
                ),
              )
              .toList(),
          // const [
          // PopupMenuItem(value: SystemActions.logs, child: Text('Logs')),
          // ],
        ),
      ],
      bottom: tabBar,
    );
  }

  Row _connectionPageTitle() {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(device.chosenName),
        SizedBox(width: 18),
        ConnectionStatusDot(device: device),
      ],
    );
  }

  @override
  Size get preferredSize => Size.fromHeight(
    kToolbarHeight + (tabBar != null ? kTextTabBarHeight : 0),
  );
}

class ConnectionStatusDot extends ConsumerWidget {
  const ConnectionStatusDot({super.key, required this.device});

  final BluetoothDevice device;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    bool isConnected =
        ref.watch(connectionManagerProvider(device)) ==
        ConnectionTransitionState.connected;
    return Container(
      width: 10,
      height: 10,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: isConnected ? Colors.green : Colors.red,
      ),
    );
  }
}
