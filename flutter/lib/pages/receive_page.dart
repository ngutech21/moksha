import 'package:cashurs_wallet/ffi.dart';
import 'package:flutter/material.dart';

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
                // FIXME add error handling
                var amountImported = await api.importToken(token: token);
                if (!context.mounted) return;
                ScaffoldMessenger.of(context).showSnackBar(SnackBar(
                  content:
                      Column(children: [Text('Imported $amountImported sats')]),
                  showCloseIcon: true,
                ));
              },
              child: const Text("Import token"))
        ],
      )),
    );
  }
}
