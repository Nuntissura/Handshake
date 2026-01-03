we recently have revalidated a lot of work packets that were in the done section of the task board.
Many failed for various reasons, so we had to move them back to ready for dev.
what of the failed work packets should we revisit first + reason. 
perhaps high priority ones first where coding or master spec compliance is the main issue.
Propose me the first packet to revisit + reason.
Here is the breakdown of the recent validation results:

batch1:

 - WP-1-Debug-Bundle-v3: PASS (revalidation report appended) docs/task_packets/WP-1-Debug-Bundle-v3.md:322         
  - WP-1-Validator-Error-Codes-v1: PASS (revalidation report appended) docs/task_packets/WP-1-Validator-Error-Codes-    v1.md:224                                                                                                       
  - WP-1-Operator-Consoles-v2: FAIL (blocking: just post-work WP-1-Operator-Consoles-v2 fails with C701-G05         
    post_sha1 mismatch + C701-G04 window drift for src/backend/handshake_core/src/flight_recorder/duckdb.rs; report 
    appended) docs/task_packets/WP-1-Operator-Consoles-v2.md:283                                                    
  - Task Board updated: moved WP-1-Operator-Consoles-v2 back to Ready for Dev docs/TASK_BOARD.md:22   

batch2:

  - WP-1-MEX-v1.2-Runtime-v2: FAIL — just post-work WP-1-MEX-v1.2-Runtime-v2 fails phase gate (missing SKELETON     
    APPROVED marker), SPEC mismatch (packet v02.96 vs repo v02.98), packet non-ASCII; report appended at docs/      
    task_packets/WP-1-MEX-v1.2-Runtime-v2.md:258.                                                                   
  - WP-1-Terminal-LAW-v2: FAIL — same phase gate + SPEC mismatch + packet non-ASCII; also src/backend/              
    handshake_core/src/terminal.rs listed in-scope but missing; report at docs/task_packets/WP-1-Terminal-LAW-      
    v2.md:251.                                                                                                      
  - WP-1-Security-Gates-v2: FAIL — just post-work WP-1-Security-Gates-v2 fails (non-ASCII + missing COR-701 manifest    fields), SPEC mismatch; plus .unwrap() in in-scope file src/backend/handshake_core/src/terminal/redaction.rs:19;    report at docs/task_packets/WP-1-Security-Gates-v2.md:320.                                                      
  - Moved all three back to Ready for Dev in docs/TASK_BOARD.md:23.   



  Batch 3:
                                                             
  - WP-1-Capability-SSoT: FAIL — blocking: just post-work WP-1-Capability-SSoT fails C701-G05 post_sha1 mismatch for    src/backend/handshake_core/src/capabilities.rs; packet also fails validator-packet-complete (missing canonical  
    **Status:**) and is anchored to spec v02.96, not docs/SPEC_CURRENT.md v02.98. Report appended: docs/            
    task_packets/WP-1-Capability-SSoT.md:264. Task board moved back: docs/TASK_BOARD.md:26.                         
  - WP-1-LLM-Core: FAIL — blocking: gate-check fails (SKELETON appears before BOOTSTRAP) at docs/task_packets/WP-1- 
    LLM-Core.md:11 vs docs/task_packets/WP-1-LLM-Core.md:49; post-work-check also fails (non-ASCII + missing COR-701    manifest). Spec drift issues noted (CompletionRequest shape + Flight Recorder FR-EVT taxonomy vs v02.98). Report    appended: docs/task_packets/WP-1-LLM-Core.md:163. Task board moved back: docs/TASK_BOARD.md:27.                 
  - WP-1-Gate-Check-Tool: FAIL — blocking: gate-check false-positive due to unanchored /SKELETON APPROVED/i matching    prose before the skeleton section (docs/task_packets/WP-1-Gate-Check-Tool.md:18 vs docs/task_packets/WP-1-Gate- 
    Check-Tool.md:79; see scripts/validation/gate-check.mjs:23). Post-work-check also fails (non-ASCII + missing    
    COR-701 manifest). Report appended: docs/task_packets/WP-1-Gate-Check-Tool.md:148. Task board moved back: docs/ 
    TASK_BOARD.md:28.    


Batch 4: 

  - WP-1-Flight-Recorder-v2: FAIL — blocking: gate-check missing SKELETON APPROVED; COR-701 post-work-check fails   
    (non-ASCII + missing manifest fields/gates); spec pointer mismatch (v02.96 vs docs/SPEC_CURRENT.md v02.98) and  
    FR-EVT schema mismatch noted. Report appended: docs/task_packets/WP-1-Flight-Recorder-v2.md:160. Task board     
    moved back: docs/TASK_BOARD.md:29.
  - WP-1-Tokenization-Service-20251228 (Tokenization-Service-v2): FAIL — blocking: gate-check missing SKELETON      
    APPROVED; COR-701 post-work-check fails (non-ASCII + missing manifest); spec pointer mismatch; spec-to-code     
    mismatch vs v02.98 TokenizationService contract (SentencePiece required, async Tokenizer used). Report appended:
    docs/task_packets/WP-1-Tokenization-Service-20251228.md:149. Task board moved back: docs/TASK_BOARD.md:30.      
  - WP-1-Storage-Foundation-20251228 (Storage-Foundation-v2): FAIL — blocking: gate-check missing SKELETON; COR-701 
    post-work-check fails (non-ASCII + missing manifest); spec pointer mismatch; mandatory audit fails because      
    sqlx:: exists outside storage at src/backend/handshake_core/src/models.rs:10 and src/backend/handshake_core/src/
    models.rs:13. Report appended: docs/task_packets/WP-1-Storage-Foundation-20251228.md:127. Task board moved back:
    docs/TASK_BOARD.md:31.


 Batch 5 :

  - WP-1-Flight-Recorder-UI-v2: FAIL (phase gate missing “SKELETON APPROVED”; COR-701 post-work manifest missing;   
    packet non-ASCII; SPEC_CURRENT mismatch v02.93 vs v02.98; user signature field missing/pending). Report appended
    at docs/task_packets/WP-1-Flight-Recorder-UI-v2.md:137. Task board moved at docs/TASK_BOARD.md:32.
  - WP-1-Workflow-Engine-v3: FAIL (phase gate missing “SKELETON APPROVED”; COR-701 manifest missing; packet non-    
    ASCII; SPEC_CURRENT mismatch v02.93 vs v02.98; plus v02.98 [HSK-WF-003] ordering not proven by current startup  
    flow; user signature field missing). Report appended at docs/task_packets/WP-1-Workflow-Engine-v3.md:123. Task  
    board moved at docs/TASK_BOARD.md:33.
  - WP-1-ACE-Validators-v3: FAIL (gate-check passes, but COR-701 manifest missing; packet non-ASCII; SPEC_CURRENT   
    mismatch v02.93/v02.91 vs v02.98; user signature field missing; TASK_BOARD was inconsistent with packet
    history). Report appended at docs/task_packets/WP-1-ACE-Validators-v3.md:216. Task board moved at docs/
    TASK_BOARD.md:34.


• Batch 6:

  -  WP-1-AI-Job-Model-v3: FAIL (phase gate missing “SKELETON APPROVED”; COR-701 post-work manifest missing; packet  
    non-ASCII; SPEC_CURRENT mismatch v02.93 vs v02.98; plus v02.98 spot-check: JobState::Stalled exists in code but 
    not in the v02.98 normative JobState list). Report appended at docs/task_packets/WP-1-AI-Job-Model-v3.md:188.   
    Task board moved at docs/TASK_BOARD.md:35.                                                                      
  - WP-1-Storage-Abstraction-Layer-v2: FAIL (phase gate: SKELETON appears before BOOTSTRAP; COR-701 manifest        
    missing; packet non-ASCII; spec mismatch; packet already contains a prior FAIL section). Report appended at     
    docs/task_packets/WP-1-Storage-Abstraction-Layer-v2.md:201. Task board moved at docs/TASK_BOARD.md:36.          
  - WP-1-AppState-Refactoring-v2: FAIL (phase gate missing “SKELETON APPROVED”; COR-701 manifest missing; packet    
    non-ASCII; SPEC_CURRENT mismatch v02.93 vs v02.98). Report appended at docs/task_packets/WP-1-AppState-         
    Refactoring-v2.md:144. Task board moved at docs/TASK_BOARD.md:37.      



Reworked work packets since mass failed revalidation;

- WP-1-Storage-Foundation-v3
- WP-1-Workflow-Engine-v4
- WP-1-Security-Gates-v3
- WP-1-Gate-Check-Tool-v2
- WP-1-Tokenization-Service-v3
- WP-1-Flight-Recorder-v3
- WP-1-OSS-Register-Enforcement-v1
- WP-1-MEX-v1.2-Runtime-v3
- WP-1-Terminal-LAW-v3
- WP-1-Operator-Consoles-v3
