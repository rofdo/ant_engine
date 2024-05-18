# ant engine

## Ideas
Place for ideas that i should write down. More or less ordered

### Client
- main thread that ties everything together
- game thread that can receive commands and sends states through handler
- networking thread communicating with the server, keeping connection alive, can send messages passed through handler
- (second rollback thread?) that only responds to broadcasted commands?


### Server
- game thread contains true state (works as client)
- networking thread receives messages, broadcasts them to confirm. Only broadcasted commands happened
- networking 
