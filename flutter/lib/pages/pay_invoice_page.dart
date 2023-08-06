// ignore_for_file: use_build_context_synchronously

import 'package:flutter/material.dart';
import 'package:moksha_wallet/generated/bridge_definitions.dart';
import 'package:moksha_wallet/pages/util.dart';
import 'package:moksha_wallet/pages/common.dart';
import '../generated/ffi.io.dart' if (dart.library.html) '../generated/ffi.web.dart';

class PayInvoicePage extends StatefulWidget {
  const PayInvoicePage({super.key});

  @override
  State<PayInvoicePage> createState() => _PayInvoicePageState();
}

class _PayInvoicePageState extends State<PayInvoicePage> {
  String invoice = '';
  FlutterInvoice? decodedInvoice;
  MintType? selectedMintType = MintType.cashu;

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(24),
      child: Center(
          child: Column(
        children: [
          TextField(
            obscureText: false,
            maxLines: 2,
            autofocus: true,
            onChanged: (value) async {
              try {
                var decoded = api.decodeInvoice(invoice: value);
                setState(() {
                  invoice = value;
                  decodedInvoice = decoded;
                });
              } catch (e) {
                if (!context.mounted) return;
                showErrorSnackBar(context, e, 'Error decoding invoice');
              }
            },
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: 'Paste invoice',
            ),
          ),
          Visibility(
              visible: invoice.isNotEmpty, child: Text("Amount: ${decodedInvoice?.amountSats} (sats)\nExpires in: ${decodedInvoice?.expiryTime} (seconds)")),
          const Spacer(),
          Visibility(
              visible: invoice.isNotEmpty,
              child: DropdownButton<MintType>(
                  value: selectedMintType,
                  items: const [
                    DropdownMenuItem(value: MintType.cashu, child: Text("Cashu")),
                    DropdownMenuItem(value: MintType.fedimint, child: Text("Fedimint"))
                  ],
                  onChanged: (value) {
                    setState(() {
                      selectedMintType = value;
                    });
                  })),
          ElevatedButton(
              onPressed: () async {
                try {
                  if (selectedMintType == null) {
                    return;
                  }

                  var paid = false;
                  if (selectedMintType == MintType.cashu) {
                    paid = await api.cashuPayInvoice(invoice: invoice).first;
                  } else if (selectedMintType == MintType.fedimint) {
                    paid = await api.fedimintPayInvoice(invoice: invoice).first;
                  } else {
                    throw Exception("Unknown mint type");
                  }

                  if (!context.mounted) return;
                  ScaffoldMessenger.of(context).showSnackBar(SnackBar(
                    content: Column(children: [paid ? const Text('Invoice has been paid: Tokens melted successfully') : const Text('Error paying invoice')]),
                    showCloseIcon: true,
                  ));
                } catch (e) {
                  if (!context.mounted) return;
                  showErrorSnackBar(context, e, 'Error paying invoice');
                }
              },
              child: const Text("Pay invoice"))
        ],
      )),
    );
  }
}
