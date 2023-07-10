import 'package:flutter/material.dart';

void showErrorSnackBar(BuildContext context, Object e, String msg) {
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(
    duration: const Duration(seconds: 5),
    content: Column(children: [Text('$msg\nError:$e')]),
    showCloseIcon: true,
  ));
}
