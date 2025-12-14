//! Soul-0 CLI - Slice 1-5
//!
//! Usage:
//!   soul0 --text "your text here"           # Single evaluation
//!   soul0 --interactive                     # Interactive solo mode
//!   soul0 --duo                             # Interactive duo mode (A: / B:)
//!   soul0 --serve                           # HTTP API server
//!   soul0 --text "text" --json              # JSON output

use clap::Parser;
use std::io::{self, BufRead, Write};

use soul0::core::{RParser, FacelockEngine, DcParser, ProofGenerator, SnapshotGenerator, save_snapshot, run_server};
use soul0::types::{FacelockState, Turn, ConversationWindow, DcResult};
use soul0::VERSION;

#[derive(Parser, Debug)]
#[command(
    name = "soul0",
    version = VERSION,
    about = "PhaseLock Soul-0 - Measure ego noise and track alignment state",
    long_about = "Soul-0 is the reference implementation of the PhaseLock protocol.\n\n\
                  It measures 'r' (narrative pressure / ego noise) from text input\n\
                  and tracks state transitions toward Facelock (aligned perception).\n\n\
                  Modes:\n  \
                  --interactive  Solo mode (r only)\n  \
                  --duo          Duo mode (r + ΔC, use A: and B: prefixes)\n  \
                  --serve        HTTP API server mode\n\n\
                  States:\n  \
                  WAITING     - Not enough data yet\n  \
                  APPROACHING - Moving toward alignment\n  \
                  LOCKED      - Full alignment, proof available\n  \
                  DRIFT       - Alignment lost"
)]
struct Args {
    /// Text to evaluate (single mode)
    #[arg(short, long)]
    text: Option<String>,
    
    /// Interactive solo mode - read lines from stdin (r only)
    #[arg(short, long)]
    interactive: bool,
    
    /// Duo mode - two speakers with A: and B: prefixes (r + ΔC)
    #[arg(short, long)]
    duo: bool,
    
    /// Run as HTTP API server
    #[arg(short, long)]
    serve: bool,
    
    /// Server address (default: 127.0.0.1:3000)
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
    
    /// Output as JSON
    #[arg(long)]
    json: bool,
    
    /// Disable colors in output
    #[arg(long)]
    no_color: bool,
    
    /// Show signal breakdown
    #[arg(long)]
    verbose: bool,
    
    /// Directory for snapshots (default: ./snapshots)
    #[arg(long, default_value = "./snapshots")]
    snapshot_dir: String,
    
    /// Disable automatic snapshot generation
    #[arg(long)]
    no_snapshot: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    
    if args.serve {
        run_serve(&args).await;
    } else if args.duo {
        run_duo(&args);
    } else if args.interactive {
        run_interactive(&args);
    } else if let Some(ref text) = args.text {
        run_single(text, &args);
    } else {
        // Default to interactive if no mode specified
        run_interactive(&args);
    }
}

/// Run single text evaluation
fn run_single(text: &str, args: &Args) {
    let parser = RParser::new();
    let mut engine = FacelockEngine::new();
    
    let r_value = parser.parse(text);
    let output = engine.update(r_value.value);
    
    if args.json {
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else if args.verbose {
        print_verbose_solo(&r_value, &output, args.no_color);
    } else {
        if args.no_color {
            println!("{}", output.to_parseable_string());
        } else {
            println!("{}", output.to_terminal_string());
        }
    }
}

/// Run interactive solo mode (Slice 1)
fn run_interactive(args: &Args) {
    let parser = RParser::new();
    let mut engine = FacelockEngine::new();
    
    print_header("Solo Mode", args.no_color);
    println!("Type text and press Enter to measure r. Type 'quit' to exit.");
    println!("Goal: reach LOCKED state (r < 0.15 for 8 seconds)");
    println!();
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    loop {
        let prompt = format_prompt_solo(&engine, args.no_color);
        print!("{}", prompt);
        stdout.flush().unwrap();
        
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        
        let line = line.trim();
        if line.eq_ignore_ascii_case("quit") || line.eq_ignore_ascii_case("exit") {
            println!("\nSession ended. Updates: {}", engine.update_count());
            break;
        }
        if line.is_empty() {
            continue;
        }
        
        let r_value = parser.parse(line);
        let output = engine.update(r_value.value);
        
        if args.json {
            println!("{}", serde_json::to_string(&output).unwrap());
        } else if args.verbose {
            print_verbose_solo(&r_value, &output, args.no_color);
        } else if args.no_color {
            println!("{}", output.to_parseable_string());
        } else {
            println!("{}", output.to_terminal_string());
            print_state_message(&output, false);
        }
    }
}

/// Run duo mode (Slice 2+3+4) - two speakers with A: and B: prefixes
fn run_duo(args: &Args) {
    let r_parser = RParser::new();
    let dc_parser = DcParser::new();
    let mut engine = FacelockEngine::new();
    let mut window = ConversationWindow::new();
    let proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    
    // Generate session ID
    let session_id: [u8; 16] = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut id = [0u8; 16];
        id[0..16].copy_from_slice(&nanos.to_le_bytes()[0..16]);
        id
    };
    
    // Track if we've generated a proof this session
    let mut proof_generated = false;
    let mut observers: Vec<String> = Vec::new();
    
    print_header("Duo Mode", args.no_color);
    println!("Two speakers mode. Prefix each line with A: or B:");
    println!("Example: A: The sky is blue");
    println!("         B: Yes, very blue today");
    println!();
    println!("Goal: reach LOCKED state (r < 0.15 AND ΔC < 0.10 for 8 seconds)");
    if !args.no_snapshot {
        println!("Snapshots will be saved to: {}", args.snapshot_dir);
    }
    println!("Type 'quit' to exit.");
    println!();
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    loop {
        let dc_result = dc_parser.calculate(&window);
        let prompt = format_prompt_duo(&engine, &dc_result, args.no_color);
        print!("{}", prompt);
        stdout.flush().unwrap();
        
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        
        let line = line.trim();
        if line.eq_ignore_ascii_case("quit") || line.eq_ignore_ascii_case("exit") {
            println!("\nSession ended. Turns: {}", window.len());
            break;
        }
        if line.is_empty() {
            continue;
        }
        
        // Parse speaker prefix
        let (speaker, text) = parse_speaker_prefix(line);
        if speaker.is_empty() {
            println!("{}⚠ Please prefix with A: or B: (e.g., 'A: Hello'){}", 
                if args.no_color { "" } else { "\x1b[33m" },
                if args.no_color { "" } else { "\x1b[0m" });
            continue;
        }
        
        // Track observers
        if !observers.contains(&speaker) {
            observers.push(speaker.clone());
        }
        
        // Calculate r for this turn
        let r_value = r_parser.parse(text);
        
        // Add turn to window
        let turn = Turn::new(speaker.clone(), text, r_value.value);
        window.add_turn(turn);
        
        // Calculate ΔC
        let dc_result = dc_parser.calculate(&window);
        
        // Update engine with combined metric
        // In duo mode, we use the higher of r or ΔC for state transitions
        let effective_r = if let Some(dc) = dc_result.value {
            r_value.value.max(dc)
        } else {
            r_value.value
        };
        let output = engine.update(effective_r);
        
        // Print output
        if args.json {
            print_json_duo(&output, &dc_result);
        } else if args.verbose {
            print_verbose_duo(&r_value, &dc_result, &output, &speaker, args.no_color);
        } else {
            print_output_duo(&output, &dc_result, &speaker, args.no_color);
        }
        
        // Check if we should generate proof + snapshot
        // Only once per LOCKED period, after 8s stability
        if output.state == FacelockState::Locked 
            && output.stable_ms >= 8000 
            && !proof_generated 
            && dc_result.is_known() 
        {
            // Generate proof
            let proof_result = proof_gen.generate(
                session_id,
                output.state,
                output.stable_ms as f64 / 1000.0,
                output.r,
                &dc_result,
                &window,
                mock_sign,
            );
            
            if let Some(proof) = proof_result.proof {
                proof_generated = true;
                
                // Print proof
                println!();
                println!("{}╔═══════════════════════════════════════════════════════════╗{}", 
                    "\x1b[32m", "\x1b[0m");
                println!("{}║  PROOF GENERATED - 248 bytes                              ║{}", 
                    "\x1b[32m", "\x1b[0m");
                println!("{}╚═══════════════════════════════════════════════════════════╝{}", 
                    "\x1b[32m", "\x1b[0m");
                println!("  {}", &proof.to_hex()[0..64]);
                println!("  ...");
                
                // Generate snapshot (1-op-1 coupling with proof)
                if !args.no_snapshot {
                    let snap_result = snap_gen.generate(&proof, &window, observers.clone());
                    
                    if let Some(snapshot) = snap_result.snapshot {
                        match save_snapshot(&snapshot, &args.snapshot_dir) {
                            Ok(path) => {
                                println!();
                                println!("{}  SNAPSHOT SAVED: {}{}", "\x1b[36m", path, "\x1b[0m");
                                println!("{}  Themes: {:?}{}", "\x1b[90m", snapshot.seen.themes, "\x1b[0m");
                                println!("{}  Blind spots: {}{}", "\x1b[90m", snapshot.blind_spots.len(), "\x1b[0m");
                                println!("{}  Horizon items: {}{}", "\x1b[90m", snapshot.horizon.len(), "\x1b[0m");
                            }
                            Err(e) => {
                                println!("{}  Snapshot save failed: {}{}", "\x1b[31m", e, "\x1b[0m");
                            }
                        }
                    }
                }
                println!();
            }
        }
        
        // Reset proof flag if we leave LOCKED
        if output.state != FacelockState::Locked {
            proof_generated = false;
        }
    }
}

/// Mock sign function for proof generation
fn mock_sign(data: &[u8]) -> [u8; 64] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let h1: [u8; 32] = hasher.finalize().into();
    
    let mut hasher = Sha256::new();
    hasher.update(&h1);
    let h2: [u8; 32] = hasher.finalize().into();
    
    let mut sig = [0u8; 64];
    sig[0..32].copy_from_slice(&h1);
    sig[32..64].copy_from_slice(&h2);
    sig
}

/// Parse speaker prefix (A: or B:)
fn parse_speaker_prefix(line: &str) -> (String, &str) {
    let line = line.trim();
    
    // Check for A: or B: prefix (case insensitive)
    if line.len() >= 2 {
        let prefix = &line[..2].to_uppercase();
        if prefix == "A:" || prefix == "B:" {
            let speaker = prefix[..1].to_string();
            let text = line[2..].trim();
            return (speaker, text);
        }
    }
    
    // Also accept "A :" or "B :" with space
    if line.len() >= 3 {
        let first_char = line.chars().next().unwrap().to_ascii_uppercase();
        if (first_char == 'A' || first_char == 'B') && line[1..].trim_start().starts_with(':') {
            let colon_pos = line.find(':').unwrap();
            let text = line[colon_pos + 1..].trim();
            return (first_char.to_string(), text);
        }
    }
    
    (String::new(), line)
}

/// Print header
fn print_header(mode: &str, no_color: bool) {
    if no_color {
        println!("========================================");
        println!("  Soul-0 v{} - {}", VERSION, mode);
        println!("========================================");
    } else {
        println!("\x1b[1m╔════════════════════════════════════════════════════════════╗\x1b[0m");
        println!("\x1b[1m║           Soul-0 v{} - {}                        ║\x1b[0m", VERSION, mode);
        println!("\x1b[1m╚════════════════════════════════════════════════════════════╝\x1b[0m");
    }
    println!();
}

/// Format solo mode prompt
fn format_prompt_solo(engine: &FacelockEngine, no_color: bool) -> String {
    let state = engine.state();
    if no_color {
        format!("[{}] > ", state)
    } else {
        format!(
            "{}{} [{}]{} > ",
            state.color_code(),
            state.emoji(),
            state,
            FacelockState::color_reset()
        )
    }
}

/// Format duo mode prompt
fn format_prompt_duo(engine: &FacelockEngine, dc: &DcResult, no_color: bool) -> String {
    let state = engine.state();
    let dc_str = dc.display_value();
    
    if no_color {
        format!("[{} | ΔC={}] > ", state, dc_str)
    } else {
        format!(
            "{}{} [{} | ΔC={}]{} > ",
            state.color_code(),
            state.emoji(),
            state,
            dc_str,
            FacelockState::color_reset()
        )
    }
}

/// Print duo mode output
fn print_output_duo(output: &soul0::types::StateOutput, dc: &DcResult, speaker: &str, no_color: bool) {
    let color = if no_color { "" } else { output.state.color_code() };
    let reset = if no_color { "" } else { FacelockState::color_reset() };
    let emoji = if no_color { "" } else { output.state.emoji() };
    
    println!(
        "{}{} [{}] r={:.3} | ΔC={} | state={} | stable={:.1}s{}",
        color,
        emoji,
        speaker,
        output.r,
        dc.display_value(),
        output.state,
        output.stable_ms as f64 / 1000.0,
        reset
    );
    
    // Print ΔC reason if UNKNOWN
    if !dc.is_known() {
        println!("{}  └─ {}{}", 
            if no_color { "" } else { "\x1b[90m" },
            dc.reason,
            reset);
    }
    
    print_state_message(output, true);
}

/// Print state transition messages
fn print_state_message(output: &soul0::types::StateOutput, is_duo: bool) {
    match output.state {
        FacelockState::Locked => {
            let extra = if is_duo { " (r + ΔC)" } else { "" };
            println!("\x1b[32m  ✓ FACELOCK ACHIEVED{} - Proof available\x1b[0m", extra);
        }
        FacelockState::Drift => {
            if output.reason == soul0::types::ReasonCode::R005_TRANSITION_TO_DRIFT {
                println!("\x1b[31m  ⚠ Alignment lost - return to stillness\x1b[0m");
            }
        }
        _ => {}
    }
}

/// Print JSON output for duo mode
fn print_json_duo(output: &soul0::types::StateOutput, dc: &DcResult) {
    #[derive(serde::Serialize)]
    struct DuoOutput<'a> {
        state: &'a soul0::types::StateOutput,
        dc: &'a DcResult,
    }
    
    let duo = DuoOutput { state: output, dc };
    println!("{}", serde_json::to_string(&duo).unwrap());
}

/// Print verbose solo output
fn print_verbose_solo(r_value: &soul0::types::RValue, output: &soul0::types::StateOutput, no_color: bool) {
    let color = if no_color { "" } else { output.state.color_code() };
    let reset = if no_color { "" } else { FacelockState::color_reset() };
    
    println!("{}┌─────────────────────────────────────┐{}", color, reset);
    println!("{}│ r = {:.4}  ({} words, {:.0}% confidence){}",
        color, r_value.value, r_value.word_count, r_value.confidence * 100.0, reset);
    println!("{}├─────────────────────────────────────┤{}", color, reset);
    println!("{}│ Signals:{}                          ", color, reset);
    println!("{}│   first_person:  {:.4} (w=2.8){}", color, r_value.signals.first_person, reset);
    println!("{}│   absolutes:     {:.4} (w=3.1){}", color, r_value.signals.absolutes, reset);
    println!("{}│   future:        {:.4} (w=2.4){}", color, r_value.signals.future_projection, reset);
    println!("{}│   past:          {:.4} (w=1.9){}", color, r_value.signals.past_attachment, reset);
    println!("{}│   comparison:    {:.4} (w=2.2){}", color, r_value.signals.comparison, reset);
    println!("{}│   judgment:      {:.4} (w=3.5){}", color, r_value.signals.judgment, reset);
    println!("{}│   urgency:       {:.4} (w=2.6){}", color, r_value.signals.urgency, reset);
    println!("{}├─────────────────────────────────────┤{}", color, reset);
    println!("{}│ State: {} | Stable: {:.1}s{}", 
        color, output.state, output.stable_ms as f64 / 1000.0, reset);
    println!("{}│ Reason: {}{}", color, output.reason.code(), reset);
    println!("{}└─────────────────────────────────────┘{}", color, reset);
}

/// Print verbose duo output
fn print_verbose_duo(
    r_value: &soul0::types::RValue, 
    dc: &DcResult, 
    output: &soul0::types::StateOutput,
    speaker: &str,
    no_color: bool
) {
    let color = if no_color { "" } else { output.state.color_code() };
    let reset = if no_color { "" } else { FacelockState::color_reset() };
    
    println!("{}┌─────────────────────────────────────┐{}", color, reset);
    println!("{}│ Speaker: {} | r = {:.4}{}", color, speaker, r_value.value, reset);
    println!("{}├─────────────────────────────────────┤{}", color, reset);
    println!("{}│ r Signals:{}", color, reset);
    println!("{}│   first_person:  {:.4}{}", color, r_value.signals.first_person, reset);
    println!("{}│   judgment:      {:.4}{}", color, r_value.signals.judgment, reset);
    println!("{}├─────────────────────────────────────┤{}", color, reset);
    println!("{}│ ΔC = {}{}", color, dc.display_value(), reset);
    if dc.is_known() {
        println!("{}│ ΔC Signals:{}", color, reset);
        println!("{}│   thematic:      {:.4} (w=0.31){}", color, dc.signals.thematic_drift, reset);
        println!("{}│   emotional:     {:.4} (w=0.28){}", color, dc.signals.emotional_volatility, reset);
        println!("{}│   logical:       {:.4} (w=0.22){}", color, dc.signals.logical_breaks, reset);
        println!("{}│   qa_mismatch:   {:.4} (w=0.12){}", color, dc.signals.qa_mismatch, reset);
        println!("{}│   ref_decay:     {:.4} (w=0.07){}", color, dc.signals.reference_decay, reset);
    } else {
        println!("{}│   Reason: {}{}", color, dc.reason, reset);
    }
    println!("{}├─────────────────────────────────────┤{}", color, reset);
    println!("{}│ State: {} | Stable: {:.1}s{}", 
        color, output.state, output.stable_ms as f64 / 1000.0, reset);
    println!("{}│ Pairs: {} | Speakers: {}{}", 
        color, dc.pair_count, dc.speaker_count, reset);
    println!("{}└─────────────────────────────────────┘{}", color, reset);
}

/// Run HTTP API server (Slice 5)
async fn run_serve(args: &Args) {
    println!();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  🜂 PhaseLock Soul-0 API Server                            ║");
    println!("║  Version: {}                                           ║", VERSION);
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    
    if let Err(e) = run_server(&args.addr, args.snapshot_dir.clone()).await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
