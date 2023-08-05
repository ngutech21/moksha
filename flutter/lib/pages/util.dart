import 'package:flutter/material.dart';

void showErrorSnackBar(BuildContext context, Object e, String msg) {
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(
    duration: const Duration(seconds: 5),
    content: Column(children: [Text('$msg\nError:$e')]),
    showCloseIcon: true,
  ));
}

String formatSats(int sats) {
  matchFunc(Match match) => '${match[1]},';
  return sats.toString().replaceAll(',', '').replaceAllMapped(RegExp(r'(\d{1,3})(?=(\d{3})+(?!\d))'), matchFunc);
}
