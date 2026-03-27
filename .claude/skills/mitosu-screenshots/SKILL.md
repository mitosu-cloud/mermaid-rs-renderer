---
name: mitosu-screenshots
description: Use when someone mentions screenshots, reference images, mitosu screenshots, or asks to look at the latest screenshots. Loads the newest screenshots as visual context for the current task.
argument-hint: [count]
---

## What This Skill Does

Loads the most recent screenshots from `~/work/screenshots-mitosu` as visual reference for the current task. The screenshots are read into the conversation so Claude can see them and use them as context.

## Steps

1. Run `ls -t ~/work/screenshots-mitosu/` via Bash to list files sorted by modification time (newest first).
2. Filter to only image files (png, jpg, jpeg, gif, webp, bmp, tiff).
3. Take the top N files, where N is:
   - The number provided via `` argument, OR
   - **2** if no argument is given
4. Read each screenshot file using the Read tool with its full path (e.g., `~/work/screenshots-mitosu/filename.png`). This displays the image as visual context.
5. After loading the screenshots, briefly confirm which files were loaded (filename and count), then proceed with whatever task the user has asked for — using the screenshots as reference.

## Notes

- If `~/work/screenshots-mitosu/` does not exist or is empty, inform the user and ask where their screenshots are located.
- If fewer images exist than the requested count, load all available images and note how many were found.
- Do NOT copy, move, or modify the screenshot files. Only read them.
- The screenshots are reference material for the user's current task — always continue with the task after loading them.
