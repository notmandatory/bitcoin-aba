JOURNAL_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")
STEVE_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")

curl -v -X POST http://localhost:8080/journal -H "Content-Type: application/json" -d '{"id":"'"$JOURNAL_ULID"'","version":1,"action":{"AddEntity":{"entity":{"id":"'"$STEVE_ULID"'", "entity_type":"Individual", "name":"Steve Myers"}}}}'

JOURNAL_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")
BDK_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")

curl -v -X POST http://localhost:8080/journal -H "Content-Type: application/json" -d '{"id":"'"$JOURNAL_ULID"'","version":1,"action":{"AddEntity":{"entity":{"id":"'"$BDK_ULID"'", "entity_type":"Organization", "name":"BDK Software LLC"}}}}'

curl -X GET http://localhost:8080/journal -H "Content-Type: application/json"
curl -X GET http://localhost:8080/ledger/entities -H "Content-Type: application/json"

JOURNAL_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")

curl -v -X POST http://localhost:8080/journal -H "Content-Type: application/json" -d '{"id":"'"$JOURNAL_ULID"'","version":1,"action":{"AddCurrency":{"currency":{"id":840, "code":"USD", "scale":2, "name":"US Dollars"}}}}'

curl -X GET http://localhost:8080/ledger/currencies -H "Content-Type: application/json"

JOURNAL_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")
ORG_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")

curl -v -X POST http://localhost:8080/journal -H "Content-Type: application/json" -d '{"id":"'"$JOURNAL_ULID"'","version":1,"action":{"AddAccount":{"account":{"id":"'"$ORG_ULID"'","number":1,"description":"BDK Org","account_type":{"Organization":{"parent_id":null, "entity_id":"'"$BDK_ULID"'"}}}}}}'

curl -v -X GET http://localhost:8080/ledger/accounts -H "Content-Type: application/json"

JOURNAL_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")
EQUITY_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")

curl -v -X POST http://localhost:8080/journal -H "Content-Type: application/json" -d '{"id":"'"$JOURNAL_ULID"'","version":1,"action":{"AddAccount":{"account":{"id":"'"$EQUITY_ULID"'","number":3,"description":"Equity","account_type":{"LedgerAccount":{"parent_id":"'"$ORG_ULID"'"}}}}}}'

JOURNAL_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")
EQUITY_STEVE_ULID=$(curl -s -X GET http://localhost:8080/ulid -H "Content-Type: application/json")

curl -v -X POST http://localhost:8080/journal -H "Content-Type: application/json" -d '{"id":"'"$JOURNAL_ULID"'","version":1,"action":{"AddAccount":{"account":{"id":"'"$EQUITY_STEVE_ULID"'","number":100,"description":"Steve Equity","account_type":{"EquityAccount":{"parent_id":"'"$EQUITY_ULID"'", "entity_id":"'"$STEVE_ULID"'" }}}}}}'

curl -X GET http://localhost:8080/ledger/accounts