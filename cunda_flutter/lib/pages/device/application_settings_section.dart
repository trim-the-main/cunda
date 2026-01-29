import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/endpoints.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final _log = Logger('ApplicationSettingsSection');

class ApplicationSettingsSection extends ConsumerStatefulWidget {
  const ApplicationSettingsSection({super.key, required this.device});
  final BluetoothDevice device;

  @override
  ConsumerState<ApplicationSettingsSection> createState() =>
      _ApplicationSettingsSectionState();
}

/// There is some state to keep in this widget
/// The provider gives us the current settings on the remote device as an
/// AsyncValue(Settings). So it comes with its associated Loading and Error
/// states. We also have a local copy of the settings that we can modify and
/// save to the remote device.
///
/// The remote settings can asynchronously change behind our back. Do not assume
/// anything about it being the same as last time we call save or anything.
/// Handle these cases:
///     switch (remoteSettings) {
///         case AsyncLoading():
///           if (remoteSettings.hasValue) print("Saving");  //
///           else print("Loading for the first time");
///         case AsyncData():
///           return data(that);
///         case AsyncError():
///           return error(that);
///       }
///     }
///
class _ApplicationSettingsSectionState
    extends ConsumerState<ApplicationSettingsSection> {
  AsyncValue<ApplSettings> settings = const AsyncLoading();
  ApplSettings? newSettings;
  bool isSaving = false;
  ProviderSubscription? sub;

  @override
  void initState() {
    super.initState();
    settings = ref.read(applicationSettingsProvider(widget.device));
    newSettings = settings.value;
    sub = ref.listenManual<AsyncValue<ApplSettings>>(
      applicationSettingsProvider(widget.device),
      (previous, next) {
        _log.fine("Fired event $next, ${next.value}");
        setState(() {
          settings = next;
          isSaving = next.hasValue && next.isLoading;
          newSettings ??= next.value;
        });
      },
      fireImmediately: true,
    );
  }

  @override
  void dispose() {
    sub?.close();
    super.dispose();
  }

  void reload() {
    ref.invalidate(applicationSettingsProvider(widget.device));
  }

  void save() {
    if (newSettings == null) {
      throw Exception("Cannot save application settings");
    }
    ref
        .read(applicationSettingsProvider(widget.device).notifier)
        .save(newSettings!);
  }

  @override
  Widget build(BuildContext context) {
    if (newSettings == null) {
      return FetchingApplSettings();
    }

    List<Widget> children = [
      Text(
        "Application Settings".toUpperCase(),
        style: Theme.of(
          context,
        ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
      ),
      SizedBox(height: 20),
      Text("Blink Duration: ${newSettings!.ledBlinkDurationMs} ms"),

      // SizedBox(height: 12),
      Slider.adaptive(
        value: newSettings!.ledBlinkDurationMs.toDouble(),
        min: 100,
        max: 1000,
        onChanged: (value) {
          setState(() {
            newSettings = ApplSettings(ledBlinkDurationMs: value.toInt());
          });
        },
      ),
    ];
    if (newSettings != settings.value) {
      children.add(SizedBox(height: 12));
      children.add(
        Center(
          child: ElevatedButton(onPressed: reload, child: Text("Reload")),
        ),
      );
      children.add(SizedBox(height: 12));
      children.add(
        Center(
          child: ElevatedButton(
            onPressed: save,
            child: isSaving ? Text("Saving...") : Text("Save"),
          ),
        ),
      );
      children.add(SizedBox(height: 12));
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: children,
    );
  }
}

class FetchingApplSettings extends StatelessWidget {
  const FetchingApplSettings({super.key});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          "Application Settings".toUpperCase(),
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
        ),
        SizedBox(height: 20),

        Center(child: CircularProgressIndicator()),
      ],
    );
  }
}
