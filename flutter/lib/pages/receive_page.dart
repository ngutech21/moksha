import 'package:flutter/material.dart';

class ReceivePage extends StatelessWidget {
  const ReceivePage({super.key});

  @override
  Widget build(BuildContext context) {
    return Center(
        child: Column(
      children: [
        const TextField(
          obscureText: true,
          decoration: InputDecoration(
            border: OutlineInputBorder(),
            labelText: 'Paste token',
          ),
        ),
        ElevatedButton(
            onPressed: () {
              const snackBar = SnackBar(
                content: Column(children: [Text('Imported tokens')]),
                showCloseIcon: true,
              );
              ScaffoldMessenger.of(context).showSnackBar(snackBar);
            },
            child: const Text("Import token"))
      ],
    ));
  }
}
