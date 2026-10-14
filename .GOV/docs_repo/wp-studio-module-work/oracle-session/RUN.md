HOW TO RUN THE ADOBE ORACLE SESSION (plain steps, about 1 to 2 hours, you stay at the computer)

What it does: four scripts (one per app) build small test images and text, and save the results as files.
They only write into: D:\Projects\LLM projects\Handshake\Handshake Worktrees\Handshake_Artifacts\WP-KERNEL-STUDIO\oracle-corpus\adobe-session\
They never save over your own files. Do not click inside the app while a script is running.
The scripts are in this folder (oracle-session): hsk_oracle_photoshop.jsx, hsk_oracle_illustrator.jsx, hsk_oracle_indesign.jsx, hsk_oracle_aftereffects.jsx.

BEFORE YOU START (once)
1. Close every document in Photoshop, Illustrator and InDesign. After Effects must start with an empty project.
2. Photoshop gamma check (skip it and only the gamma part is skipped):
   a. Edit > Color Settings. Make sure the top "Settings" box shows a real name (not "Custom"). If it says Custom: click Save, give it a name, OK.
   b. Click "More Options". Tick "Blend RGB Colors Using Gamma" and type 1.00. Click Save, name it exactly HSK-gamma-1.0, OK.
   c. Select your normal setting again in the Settings box, untick the gamma box if it is ticked, click OK. The script switches to HSK-gamma-1.0 by itself and switches back.
3. After Effects: Edit > Preferences > Scripting & Expressions > tick "Allow Scripts to Write Files and Access Network", OK.
4. Note: the first After Effects launch creates its own settings files (normal).

RUN, ONE APP AT A TIME (open the app once, run its script once, close the app when done)
- Photoshop: File > Scripts > Browse... and pick hsk_oracle_photoshop.jsx. It is the longest (several hundred files, maybe 30 to 60 minutes). Windows flicker; that is normal.
- Illustrator: File > Scripts > Other Script... (Ctrl+F12) and pick hsk_oracle_illustrator.jsx.
- InDesign: Window > Utilities > Scripts. In the Scripts panel right-click "User", choose "Reveal in Explorer", copy hsk_oracle_indesign.jsx into that folder, then double-click it in the panel.
- After Effects: File > Scripts > Run Script File... and pick hsk_oracle_aftereffects.jsx.
(Command-line launch forms are in SESSION_PLAN.json; they are untested, use the menus.)

WHEN A SCRIPT ENDS
- Look in the output folder, in the sub-folder of that app: manifest.json must exist. If errors.log exists, some variants failed; that is expected on the first run for Photoshop.
- To redo only some cases: create the folder "control" inside adobe-session and put a text file in it (ps_only.txt, ai_only.txt, id_only.txt or ae_only.txt) with one case id per line, for example PS-C09. Then run the script again.

HOW TO CHECK THE PHOTOSHOP SETTING WAS PUT BACK
Open manifest.json in the photoshop folder: "colorSettings_before" and "colorSettings_after" must be the same, and "restore_checked" must say true. If not, set your normal setting in Edit > Color Settings by hand.

IF SOMETHING GOES WRONG
- A dialog appears or the app hangs: do not force-close other things. Tell the lead which app and what the dialog said.
- Nothing is lost: results already written stay in the output folder.

WHAT TO TELL THE LEAD WHEN FINISHED
"Session done" plus which apps ran. The lead reads the manifests and the errors.log files; you do not need to open the images.
