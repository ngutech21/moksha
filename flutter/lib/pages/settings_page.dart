import 'package:moksha_wallet/main.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:moksha_wallet/pages/util.dart';

class SettingsPage extends ConsumerStatefulWidget {
  const SettingsPage({super.key});

  @override
  ConsumerState<ConsumerStatefulWidget> createState() => _SettingsPageState();
}

class _SettingsPageState extends ConsumerState<SettingsPage> {
  String federationConnectString = "";

  @override
  Widget build(BuildContext context) {
    AsyncValue<String> config = ref.watch(dbPathProvider);

    var dbPath = config.when(
      loading: () => const CircularProgressIndicator(),
      error: (err, stack) => Text('Error: $err'),
      data: (db) {
        return db;
      },
    );

    return Container(
      margin: const EdgeInsets.all(24),
      child: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            const Text(
              'Settings',
            ),
            const TextField(
              decoration: InputDecoration(
                border: OutlineInputBorder(),
                labelText: 'Mint URL',
              ),
            ),
            TextField(
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: 'Federation connect-string',
              ),
              onChanged: (value) => setState(() {
                federationConnectString = value;
              }),
            ),
            ElevatedButton(
                onPressed: () async {
                  try {
                    await api.joinFederation(
                        federation: federationConnectString);
                  } catch (e) {
                    showErrorSnackBar(context, e, 'Error joining federation');
                  }
                },
                child: const Text('Join')),
            Text(
              'DB Path: $dbPath',
              style: Theme.of(context).textTheme.titleSmall,
            )
          ],
        ),
      ),
    );
  }
}
