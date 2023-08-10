// ignore_for_file: use_build_context_synchronously

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:moksha_wallet/generated/bridge_definitions.dart';
import 'package:moksha_wallet/main.dart';
import 'package:moksha_wallet/pages/util.dart';
import 'package:moksha_wallet/pages/common.dart';

class PayInvoicePage extends ConsumerStatefulWidget {
  final FlutterInvoice? invoice;
  const PayInvoicePage({super.key, this.invoice});

  @override
  ConsumerState<PayInvoicePage> createState() => _PayInvoicePageState();
}

class _PayInvoicePageState extends ConsumerState<PayInvoicePage> {
  String invoice = '';
  FlutterInvoice? decodedInvoice;
  MintType? selectedMintType = MintType.cashu;

  @override
  Widget build(BuildContext context) {
    if (widget.invoice != null) {
      invoice = widget.invoice!.pr;
      decodedInvoice = widget.invoice;
    }

    return Container(
      margin: const EdgeInsets.all(24),
      child: Center(
          child: Column(
        children: [
          TextField(
            obscureText: false,
            maxLines: 2,
            autofocus: true,
            controller: TextEditingController(text: invoice),
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
                    updateCashuBalance(ref);
                  } else if (selectedMintType == MintType.fedimint) {
                    paid = await api.fedimintPayInvoice(invoice: invoice).first;
                    updateFedimintBalance(ref);
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
