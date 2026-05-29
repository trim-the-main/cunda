import 'package:flutter/material.dart';

/// Show a yes/no confirmation dialog.
///
/// Returns `true` if the user tapped the confirm button, `false` if they
/// cancelled or dismissed the dialog by tapping outside it.
///
/// When [destructive] is true, the confirm label is rendered in red.
Future<bool> showConfirmDialog({
  required BuildContext context,
  required String title,
  required String body,
  String cancelLabel = 'Cancel',
  String confirmLabel = 'Confirm',
  bool destructive = false,
}) async {
  final result = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(title),
      content: Text(body),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: Text(cancelLabel),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, true),
          child: Text(
            confirmLabel,
            style: destructive ? const TextStyle(color: Colors.red) : null,
          ),
        ),
      ],
    ),
  );
  return result ?? false;
}
