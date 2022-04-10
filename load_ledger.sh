
curl -X POST http://localhost:8080/journal -H "Content-Type: application/json" -d '{"id":"01FQTKAGR9NSS0Z6MAN8ADT9J4","version":1,"action":{"AddAccount":{"account":{"id":"01FQTKAHHWV2N8H28W62GW0ZZS","number":0,"description":"Test Account","account_type":{"Organization":{"parent_id":null, "entity_id": "01G0APXF9KV06F8B4JNAKH1QN2"}}}}}}'

curl -X POST http://localhost:8080/journal -H "Content-Type: application/json" -d '{"id":"01FQTKAKP33QJ6X88CP4SYTGT1","version":1,"action":{"AddAccount":{"account":{"id":"01FQR4BCFF55PZYS6XXEKTEHVZ","number":0,"description":"Test Account","account_type":{"LedgerAccount":{"parent_id":"01FQTKAHHWV2N8H28W62GW0ZZS", "currency_id":840 }}}}}}'

curl -X GET http://localhost:8080/journal