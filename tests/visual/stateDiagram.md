## Basic State Diagram

```mermaid
stateDiagram-v2
    [*] --> Still
    Still --> Moving
    Moving --> Still
    Moving --> Crash
    Crash --> [*]
```

## Simple State Declaration

```mermaid
stateDiagram-v2
    s1
```

## State Description Using State Keyword

```mermaid
stateDiagram-v2
    state "This is a state" as s2
```

## State Description Using Colon Syntax

```mermaid
stateDiagram-v2
    s2 : This is a state
```

## Transition With Label

```mermaid
stateDiagram-v2
    [*] --> s1
    s1 --> s2: A transition
    s2 --> [*]
```

## Start and End States

```mermaid
stateDiagram-v2
    [*] --> s1
    s1 --> [*]
```

## Composite States

```mermaid
stateDiagram-v2
    [*] --> First
    state First {
        [*] --> second
        second --> [*]
    }
    First --> End
    state End {
        [*] --> third
        third --> [*]
    }
    End --> [*]
```

## Nested Composite States

```mermaid
stateDiagram-v2
    [*] --> First
    state First {
        [*] --> Second
        state Second {
            [*] --> second
            second --> Third
            state Third {
                [*] --> third
                third --> [*]
            }
            Third --> [*]
        }
        Second --> [*]
    }
    First --> End
    state End {
        [*] --> second
        second --> [*]
    }
    End --> [*]
```

## Transitions Between Composite States

```mermaid
stateDiagram-v2
    state First {
        [*] --> fir
        fir --> [*]
    }
    state Second {
        [*] --> sec
        sec --> [*]
    }
    First --> Second
```

## Choice Pseudostate

```mermaid
stateDiagram-v2
    state if_state <<choice>>
    [*] --> if_state
    if_state --> sw1: if value1
    if_state --> sw2: if value2
    sw1 --> end
    sw2 --> end
    end --> [*]
```

## Fork and Join

```mermaid
stateDiagram-v2
    state fork_state <<fork>>
      [*] --> fork_state
      fork_state --> State2
      fork_state --> State3
      state join_state <<join>>
      State2 --> join_state
      State3 --> join_state
      join_state --> [*]
```

## Notes on States

```mermaid
stateDiagram-v2
    State1: The state with a note
    note right of State1
        Important information! You can write
        notes.
    end note
    State1 --> State2
    note left of State2 : This is a note on State2.
```

## Concurrency

```mermaid
stateDiagram-v2
    [*] --> Active
    state Active {
        [*] --> NumLockOff
        NumLockOff --> NumLockOn : EvNumLockPressed
        NumLockOn --> NumLockOff : EvNumLockPressed
        --
        [*] --> CapsLockOff
        CapsLockOff --> CapsLockOn : EvCapsLockPressed
        CapsLockOn --> CapsLockOff : EvCapsLockPressed
        --
        [*] --> ScrollLockOff
        ScrollLockOff --> ScrollLockOn : EvScrollLockPressed
        ScrollLockOn --> ScrollLockOff : EvScrollLockPressed
    }
```

## Direction Left to Right

```mermaid
stateDiagram-v2
    direction LR
    [*] --> A
    A --> B
    B --> [*]
```

## Direction Left to Right With Transitions

```mermaid
stateDiagram-v2
    direction LR
    [*] --> Still
    Still --> Moving
    Moving --> Still
    Moving --> Crash
    Crash --> [*]
```

## Comments

```mermaid
stateDiagram-v2
    [*] --> s1
    s1 --> [*]
    %% This is a comment
```

## ClassDef Styling

```mermaid
stateDiagram-v2
    [*] --> Moving
    state Moving {
        [*] --> fast
        fast --> slow
        slow --> [*]
    }
    Moving --> [*]
    classDef movement font-style:italic;
    classDef badBadEvent fill:#f00,color:white,font-weight:bold,stroke-width:2px,stroke:yellow
    class Moving movement
    class Crash badBadEvent
```

## Applying Styles With Triple Colon Operator

```mermaid
stateDiagram-v2
    [*] --> s1 :::someclass
    s1 --> [*]
    classDef someclass fill:#f96
```

## State With Spaces Using ID Reference

```mermaid
stateDiagram-v2
    [*] --> yswsii
    state "Your state with spaces in it" as yswsii
    yswsii --> YetAnotherState
    state "Another State" as YetAnotherState
    YetAnotherState --> [*]
    classDef movement font-style:italic
    class yswsii movement
```
