## Demo

```mermaid
zenuml
title Demo
Alice->John: Hello John, how are you?
John->Alice: Great!
Alice->John: See you later!
```

## Declare Participant

```mermaid
zenuml
title Declare participant (optional)
Bob
Alice
Alice->Bob: Hi Bob
Bob->Alice: Hi Alice
```

## Annotators

```mermaid
zenuml
title Annotators
@Actor Alice
@Database Bob
Alice->Bob: Hi Bob
Bob->Alice: Hi Alice
```

## Aliases

```mermaid
zenuml
title Aliases
A as Alice
J as John
A->J: Hello John, how are you?
J->A: Great!
```

## Sync Message

```mermaid
zenuml
title Sync message
A.SyncMessage
A.SyncMessage(with, parameters) {
  B.nestedSyncMessage()
}
```

## Async Message

```mermaid
zenuml
title Async message
Alice->Bob: How are you?
```

## Creation Message

```mermaid
zenuml
new A1
new A2(with, parameters)
```

## Reply Message

```mermaid
zenuml
// 1. assign a variable from a sync message.
a = A.SyncMessage()

// 1.1. optionally give the variable a type
SomeType a = A.SyncMessage()

// 2. use return keyword
A.SyncMessage() {
return result
}

// 3. use @return or @reply annotator on an async message
@return
A->B: result
```

## Reply Message Advanced

```mermaid
zenuml
title Reply message
Client->A.method() {
  B.method() {
    if(condition) {
      return x1
      // return early
      @return
      A->Client: x11
    }
  }
  return x2
}
```

## Nesting

```mermaid
zenuml
A.method() {
  B.nested_sync_method()
  B->C: nested async message
}
```

## Comments

```mermaid
zenuml
// a comment on a participant will not be rendered
BookService
// a comment on a message.
// **Markdown** is supported.
BookService.getBook()
```

## Loops

```mermaid
zenuml
Alice->John: Hello John, how are you?
while(true) {
  John->Alice: Great!
}
```

## Alt

```mermaid
zenuml
Alice->Bob: Hello Bob, how are you?
if(is_sick) {
  Bob->Alice: Not so good :(
} else {
  Bob->Alice: Feeling fresh like a daisy
}
```

## Opt

```mermaid
zenuml
Alice->Bob: Hello Bob, how are you?
Bob->Alice: Not so good :(
opt {
  Bob->Alice: Thanks for asking
}
```

## Parallel

```mermaid
zenuml
par {
    Alice->Bob: Hello guys!
    Alice->John: Hello guys!
}
```

## Try Catch Finally

```mermaid
zenuml
try {
  Consumer->API: Book something
  API->BookingService: Start booking process
} catch {
  API->Consumer: show failure
} finally {
  API->BookingService: rollback status
}
```
