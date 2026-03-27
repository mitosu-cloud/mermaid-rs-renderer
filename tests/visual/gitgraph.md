## Example Git Diagram

```mermaid
---
title: Example Git diagram
---
gitGraph
   commit
   commit
   branch develop
   checkout develop
   commit
   commit
   checkout main
   merge develop
   commit
   commit
```

## Simple Three Commits

```mermaid
gitGraph
   commit
   commit
   commit
```

## Custom Commit IDs

```mermaid
gitGraph
   commit id: "Alpha"
   commit id: "Beta"
   commit id: "Gamma"
```

## Different Commit Types

```mermaid
gitGraph
   commit id: "Normal"
   commit
   commit id: "Reverse" type: REVERSE
   commit
   commit id: "Highlight" type: HIGHLIGHT
   commit
```

## Commits with Tags

```mermaid
gitGraph
   commit
   commit id: "Normal" tag: "v1.0.0"
   commit
   commit id: "Reverse" type: REVERSE tag: "RC_1"
   commit
   commit id: "Highlight" type: HIGHLIGHT tag: "8.8.4"
   commit
```

## Create New Branch

```mermaid
gitGraph
   commit
   commit
   branch develop
   commit
   commit
   commit
```

## Checkout Existing Branch

```mermaid
gitGraph
   commit
   commit
   branch develop
   commit
   commit
   commit
   checkout main
   commit
   commit
```

## Merging Branches

```mermaid
gitGraph
   commit
   commit
   branch develop
   commit
   commit
   commit
   checkout main
   commit
   commit
   merge develop
   commit
   commit
```

## Merge with Custom Attributes

```mermaid
gitGraph
   commit id: "1"
   commit id: "2"
   branch nice_feature
   checkout nice_feature
   commit id: "3"
   checkout main
   commit id: "4"
   checkout nice_feature
   branch very_nice_feature
   checkout very_nice_feature
   commit id: "5"
   checkout main
   commit id: "6"
   checkout nice_feature
   commit id: "7"
   checkout main
   merge nice_feature id: "customID" tag: "customTag" type: REVERSE
   checkout very_nice_feature
   commit id: "8"
   checkout main
   commit id: "9"
```

## Cherry-Pick with Merge Commit Parent

```mermaid
gitGraph
   commit id: "ZERO"
   branch develop
   branch release
   commit id:"A"
   checkout main
   commit id:"ONE"
   checkout develop
   commit id:"B"
   checkout main
   merge develop id:"MERGE"
   commit id:"TWO"
   checkout release
   cherry-pick id:"MERGE" parent:"B"
   commit id:"THREE"
   checkout develop
   commit id:"C"
```

## Hide Branch Names

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'base'
  gitGraph:
    showBranches: false
---
gitGraph
   commit
   branch hotfix
   checkout hotfix
   commit
   branch develop
   checkout develop
   commit id:"ash" tag:"abc"
   branch featureB
   checkout featureB
   commit type:HIGHLIGHT
   checkout main
   checkout hotfix
   commit type:NORMAL
   checkout develop
   commit type:REVERSE
   checkout featureB
   commit
   checkout main
   merge hotfix
   checkout featureB
   commit
   checkout develop
   branch featureA
   commit
   checkout develop
   merge hotfix
   checkout featureA
   commit
   checkout featureB
   commit
   checkout develop
   merge featureA
   branch release
   checkout release
   commit
   checkout main
   commit
   checkout release
   merge main
   checkout develop
   merge release
```

## Rotated Commit Labels

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'base'
  gitGraph:
    rotateCommitLabel: true
---
gitGraph
   commit id: "feat(api): ..."
   commit id: "a"
   commit id: "b"
   commit id: "fix(client): .extra long label.."
   branch c2
   commit id: "feat(modules): ..."
   commit id: "test(client): ..."
   checkout main
   commit id: "fix(api): ..."
   commit id: "ci: ..."
   branch b1
   commit
   branch b2
   commit
```

## Horizontal Commit Labels

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'base'
  gitGraph:
    rotateCommitLabel: false
---
gitGraph
   commit id: "feat(api): ..."
   commit id: "a"
   commit id: "b"
   commit id: "fix(client): .extra long label.."
   branch c2
   commit id: "feat(modules): ..."
   commit id: "test(client): ..."
   checkout main
   commit id: "fix(api): ..."
   commit id: "ci: ..."
   branch b1
   commit
   branch b2
   commit
```

## Hide Commit Labels

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'base'
  gitGraph:
    showBranches: false
    showCommitLabel: false
---
gitGraph
   commit
   branch hotfix
   checkout hotfix
   commit
   branch develop
   checkout develop
   commit id:"ash"
   branch featureB
   checkout featureB
   commit type:HIGHLIGHT
   checkout main
   checkout hotfix
   commit type:NORMAL
   checkout develop
   commit type:REVERSE
   checkout featureB
   commit
   checkout main
   merge hotfix
   checkout featureB
   commit
   checkout develop
   branch featureA
   commit
   checkout develop
   merge hotfix
   checkout featureA
   commit
   checkout featureB
   commit
   checkout develop
   merge featureA
   branch release
   checkout release
   commit
   checkout main
   commit
   checkout release
   merge main
   checkout develop
   merge release
```

## Custom Main Branch Name

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'base'
  gitGraph:
    showBranches: true
    showCommitLabel: true
    mainBranchName: 'MetroLine1'
---
gitGraph
   commit id:"NewYork"
   commit id:"Dallas"
   branch MetroLine2
   commit id:"LosAngeles"
   commit id:"Chicago"
   commit id:"Houston"
   branch MetroLine3
   commit id:"Phoenix"
   commit type: HIGHLIGHT id:"Denver"
   commit id:"Boston"
   checkout MetroLine1
   commit id:"Atlanta"
   merge MetroLine3
   commit id:"Miami"
   commit id:"Washington"
   merge MetroLine2 tag:"MY JUNCTION"
   commit id:"Boston"
   commit id:"Detroit"
   commit type:REVERSE id:"SanFrancisco"
```

## Branch Ordering with Custom Order

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'base'
  gitGraph:
    showBranches: true
    showCommitLabel: true
---
gitGraph
   commit
   branch test1 order: 3
   branch test2 order: 2
   branch test3 order: 1
```

## Branch Ordering with Main Branch Order

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'base'
  gitGraph:
    showBranches: true
    showCommitLabel: true
    mainBranchOrder: 2
---
gitGraph
   commit
   branch test1 order: 3
   branch test2
   branch test3
   branch test4 order: 1
```

## Left to Right Orientation

```mermaid
gitGraph LR:
   commit
   commit
   branch develop
   commit
   commit
   checkout main
   commit
   commit
   merge develop
   commit
   commit
```

## Top to Bottom Orientation

```mermaid
gitGraph TB:
   commit
   commit
   branch develop
   commit
   commit
   checkout main
   commit
   commit
   merge develop
   commit
   commit
```

## Bottom to Top Orientation

```mermaid
gitGraph BT:
   commit
   commit
   branch develop
   commit
   commit
   checkout main
   commit
   commit
   merge develop
   commit
   commit
```

## Temporal Commits (Default)

```mermaid
---
config:
  gitGraph:
    parallelCommits: false
---
gitGraph:
   commit
   branch develop
   commit
   commit
   checkout main
   commit
   commit
```

## Parallel Commits Enabled

```mermaid
---
config:
  gitGraph:
    parallelCommits: true
---
gitGraph:
   commit
   branch develop
   commit
   commit
   checkout main
   commit
   commit
```

## Base Theme

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'base'
---
gitGraph
   commit
   branch hotfix
   checkout hotfix
   commit
   branch develop
   checkout develop
   commit id:"ash" tag:"abc"
   branch featureB
   checkout featureB
   commit type:HIGHLIGHT
   checkout main
   checkout hotfix
   commit type:NORMAL
   checkout develop
   commit type:REVERSE
   checkout featureB
   commit
   checkout main
   merge hotfix
   checkout featureB
   commit
   checkout develop
   branch featureA
   commit
   checkout develop
   merge hotfix
   checkout featureA
   commit
   checkout featureB
   commit
   checkout develop
   merge featureA
   branch release
   checkout release
   commit
   checkout main
   commit
   checkout release
   merge main
   checkout develop
   merge release
```

## Forest Theme

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'forest'
---
gitGraph
   commit
   branch hotfix
   checkout hotfix
   commit
   branch develop
   checkout develop
   commit id:"ash" tag:"abc"
   branch featureB
   checkout featureB
   commit type:HIGHLIGHT
   checkout main
   checkout hotfix
   commit type:NORMAL
   checkout develop
   commit type:REVERSE
   checkout featureB
   commit
   checkout main
   merge hotfix
   checkout featureB
   commit
   checkout develop
   branch featureA
   commit
   checkout develop
   merge hotfix
   checkout featureA
   commit
   checkout featureB
   commit
   checkout develop
   merge featureA
   branch release
   checkout release
   commit
   checkout main
   commit
   checkout release
   merge main
   checkout develop
   merge release
```

## Default Theme

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'default'
---
gitGraph
   commit type:HIGHLIGHT
   branch hotfix
   checkout hotfix
   commit
   branch develop
   checkout develop
   commit id:"ash" tag:"abc"
   branch featureB
   checkout featureB
   commit type:HIGHLIGHT
   checkout main
   checkout hotfix
   commit type:NORMAL
   checkout develop
   commit type:REVERSE
   checkout featureB
   commit
   checkout main
   merge hotfix
   checkout featureB
   commit
   checkout develop
   branch featureA
   commit
   checkout develop
   merge hotfix
   checkout featureA
   commit
   checkout featureB
   commit
   checkout develop
   merge featureA
   branch release
   checkout release
   commit
   checkout main
   commit
   checkout release
   merge main
   checkout develop
   merge release
```

## Dark Theme

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'dark'
---
gitGraph
   commit
   branch hotfix
   checkout hotfix
   commit
   branch develop
   checkout develop
   commit id:"ash" tag:"abc"
   branch featureB
   checkout featureB
   commit type:HIGHLIGHT
   checkout main
   checkout hotfix
   commit type:NORMAL
   checkout develop
   commit type:REVERSE
   checkout featureB
   commit
   checkout main
   merge hotfix
   checkout featureB
   commit
   checkout develop
   branch featureA
   commit
   checkout develop
   merge hotfix
   checkout featureA
   commit
   checkout featureB
   commit
   checkout develop
   merge featureA
   branch release
   checkout release
   commit
   checkout main
   commit
   checkout release
   merge main
   checkout develop
   merge release
```

## Neutral Theme

```mermaid
---
config:
  logLevel: 'debug'
  theme: 'neutral'
---
gitGraph
   commit
   branch hotfix
   checkout hotfix
   commit
   branch develop
   checkout develop
   commit id:"ash" tag:"abc"
   branch featureB
   checkout featureB
   commit type:HIGHLIGHT
   checkout main
   checkout hotfix
   commit type:NORMAL
   checkout develop
   commit type:REVERSE
   checkout featureB
   commit
   checkout main
   merge hotfix
   checkout featureB
   commit
   checkout develop
   branch featureA
   commit
   checkout develop
   merge hotfix
   checkout featureA
   commit
   checkout featureB
   commit
   checkout develop
   merge featureA
   branch release
   checkout release
   commit
   checkout main
   commit
   checkout release
   merge main
   checkout develop
   merge release
```

## Customizing Branch Colors

```mermaid
%%{init: { 'logLevel': 'debug', 'theme': 'base', 'gitGraph': {'mainBranchName': 'main', 'mainBranchOrder': 0}, 'themeVariables': { 'git0': '#ff0000', 'git1': '#00ff00', 'git2': '#0000ff', 'git3': '#ffff00'}} }%%
gitGraph
   commit id: "1"
   commit id: "2"
   branch develop
   commit id: "3"
   commit id: "4"
   checkout main
   commit id: "5"
   merge develop
```

## Customizing Branch Label Colors

```mermaid
%%{init: { 'logLevel': 'debug', 'theme': 'base', 'themeVariables': { 'gitBranchLabel0': '#ffffff', 'gitBranchLabel1': '#000000', 'gitBranchLabel2': '#ffff00'}} }%%
gitGraph
   commit id: "1"
   commit id: "2"
   branch develop
   commit id: "3"
   commit id: "4"
   checkout main
   commit id: "5"
   merge develop
   branch branch1
   commit id: "6"
   branch branch2
   commit id: "7"
   branch test4
   commit id: "8"
```

## Customizing Commit Colors

```mermaid
%%{init: { 'logLevel': 'debug', 'theme': 'base', 'themeVariables': { 'commitLabelColor': '#ff0000', 'commitLabelBackground': '#00ff00'}} }%%
gitGraph
   commit id: "1"
   commit id: "2"
   branch develop
   commit id: "3"
```

## Customizing Commit Label Font Size

```mermaid
%%{init: { 'logLevel': 'debug', 'theme': 'base', 'themeVariables': { 'commitLabelFontSize': '16px'}} }%%
gitGraph
   commit id: "1"
   commit id: "2"
```

## Customizing Tag Label Font Size

```mermaid
%%{init: { 'logLevel': 'debug', 'theme': 'base', 'themeVariables': { 'tagLabelFontSize': '14px'}} }%%
gitGraph
   commit id: "1" tag: "v1.0"
   commit id: "2" tag: "v1.1"
```

## Customizing Tag Colors

```mermaid
%%{init: { 'logLevel': 'debug', 'theme': 'base', 'themeVariables': { 'tagLabelColor': '#ffffff', 'tagLabelBackground': '#ff0000', 'tagLabelBorder': '#000000'}} }%%
gitGraph
   commit id: "1" tag: "v1.0"
   commit id: "2" tag: "v1.1"
```

## Customizing Highlight Commit Colors

```mermaid
%%{init: { 'logLevel': 'debug', 'theme': 'base', 'themeVariables': { 'gitInv0': '#ff0000'}} }%%
gitGraph
   commit id: "1" type: HIGHLIGHT
   commit id: "2"
   branch develop
   commit id: "3" type: HIGHLIGHT
```
