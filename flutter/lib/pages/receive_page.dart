import 'package:cashurs_wallet/ffi.dart';
import 'package:flutter/material.dart';

class ReceivePage extends StatelessWidget {
  const ReceivePage({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(24),
      child: Center(
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
              onPressed: () async {
                print("imported token");
                api.importToken(
                    token:
                        "cashuAeyJ0b2tlbiI6W3sibWludCI6Imh0dHA6Ly8xMjcuMC4wLjE6MzMzOCIsInByb29mcyI6W3siYW1vdW50IjoyLCJzZWNyZXQiOiJkTGxOaGpBNXFEY3I2cDBxVFpTUHhmZzEiLCJDIjoiMDM1ODRmZGEwNGRjNWI2NmNhYjUzZWUyYTRjMmY1ZDZiNDE1MjUyMjQ1MzM2OTlhNjdiNzgwYTQ1OTg1YmI3NGYwIiwiaWQiOiJtUjlQSjNNempMMXkifSx7ImFtb3VudCI6OCwic2VjcmV0Ijoia1B2Z2NaaUZ3VDBGR3drcEg4U21nUm4xIiwiQyI6IjAyZDdlMmJhODVjMDhlNzU1ODMzMWEzNjA1ZmY1MjhjZGViZDdkN2FlMTU0ODQyZTEwMzc3OTU2YjJlOWIyOWJjYiIsImlkIjoibVI5UEozTXpqTDF5In1dfV19");
                // const snackBar = SnackBar(
                //   content: Column(children: [Text('Imported tokens')]),
                //   showCloseIcon: true,
                // );
                // ScaffoldMessenger.of(context).showSnackBar(snackBar);
              },
              child: const Text("Import token"))
        ],
      )),
    );
  }
}
