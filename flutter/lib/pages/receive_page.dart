// ignore_for_file: use_build_context_synchronously

import 'package:moksha_wallet/ffi.dart';
import 'package:flutter/material.dart';
import 'package:moksha_wallet/pages/util.dart';

class ReceivePage extends StatefulWidget {
  const ReceivePage({super.key});

  @override
  State<ReceivePage> createState() => _ReceivePageState();
}

class _ReceivePageState extends State<ReceivePage> {
  String token = '';

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(24),
      child: Center(
          child: Column(
        children: [
          TextField(
            obscureText: false,
            maxLines: 5,
            autofocus: true,
            onChanged: (value) => setState(() {
              token = value;
            }),
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: 'Paste token',
            ),
          ),
          const Spacer(),
          ElevatedButton(
              onPressed: () async {
                try {
                  var amountImported = await api.receiveToken(token: token);

                  if (!context.mounted) return;
                  ScaffoldMessenger.of(context).showSnackBar(SnackBar(
                    content: Column(
                        children: [Text('Imported $amountImported sats')]),
                    showCloseIcon: true,
                  ));
                } catch (e) {
                  if (!context.mounted) return;
                  showErrorSnackBar(context, e, 'Error importing token');
                }
              },
              child: const Text("Import token"))
        ],
      )),
    );
  }
}
