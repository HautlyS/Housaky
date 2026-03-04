//! Quantum command handlers for Housaky CLI

use anyhow::Result;

use crate::config::Config;
use crate::commands::QuantumCommands;
use crate::quantum::circuit::{Gate, QuantumCircuit};
use crate::quantum::{
    AmazonBraketBackend, QuantumBackend, QuantumConfig, SimulatorBackend,
};

/// Create a Bell state circuit for testing
fn bell_circuit() -> QuantumCircuit {
    let mut c = QuantumCircuit::new(2);
    c.add_gate(Gate::h(0));
    c.add_gate(Gate::cnot(0, 1));
    c.measure_all();
    c
}

/// Handle quantum commands
#[allow(clippy::too_many_lines)]
pub async fn handle_quantum(config: &Config, quantum_command: QuantumCommands) -> Result<()> {
    match quantum_command {
        QuantumCommands::RunBraket {
            shots,
            device,
            bucket,
            prefix,
        } => {
            println!("Submitting Bell-state circuit to Amazon Braket...");
            println!("  Device : {device}");
            println!("  Shots  : {shots}");
            println!("  Bucket : s3://{bucket}/{prefix}");
            let cfg = QuantumConfig {
                backend: "braket".to_string(),
                shots,
                braket_device_arn: device,
                braket_s3_bucket: bucket,
                braket_s3_prefix: prefix,
                ..QuantumConfig::default()
            };
            let backend = AmazonBraketBackend::from_config(&cfg).await?;
            let circuit = bell_circuit();
            match backend.execute_circuit(&circuit).await {
                Ok(result) => {
                    println!("\nTask ARN  : {}", result.backend_id);
                    println!("Shots run : {}", result.shots);
                    println!("Runtime   : {} ms", result.execution_time_ms);
                    println!("\nCounts:");
                    let mut counts: Vec<_> = result.counts.iter().collect();
                    counts.sort_by(|a, b| b.1.cmp(a.1));
                    for (bitstring, count) in &counts {
                        let pct = (**count as f64 / result.shots as f64) * 100.0;
                        println!(
                            "  |{bitstring}> : {count:5}  ({pct:.1}%)",
                            count = **count
                        );
                    }
                }
                Err(e) => {
                    let msg = format!("{e:?}");
                    eprintln!("\n[DEBUG] Raw Braket error: {msg}\n");
                    if msg.contains("not authorized to use this resource")
                        || msg.contains("opt-in")
                        || msg.contains("not subscribed")
                        || msg.contains("not enabled")
                        || (msg.contains("AccessDeniedException") && msg.contains("contact customer support"))
                    {
                        eprintln!("❌  Amazon Braket service not yet activated on this AWS account.");
                        eprintln!("\n   To activate Braket:");
                        eprintln!("   1. Open: https://console.aws.amazon.com/braket/home");
                        eprintln!("   2. Click \"Get started\" and accept the service terms (30 sec).");
                        eprintln!("   3. Re-run this command.");
                        std::process::exit(1);
                    } else if msg.contains("AccessDeniedException")
                        || msg.contains("not authorized")
                        || msg.contains("NotAuthorized")
                    {
                        eprintln!("❌  IAM permission denied submitting Braket task.");
                        eprintln!("   Ensure your IAM user/role has the AmazonBraketFullAccess policy.");
                        eprintln!("   Also ensure the S3 bucket s3://amazon-braket-housaky-541739678328 exists.");
                        std::process::exit(1);
                    } else if msg.contains("dispatch failure")
                        || msg.contains("ConnectError")
                        || msg.contains("connection refused")
                        || msg.contains("No network")
                        || msg.contains("dns error")
                        || msg.contains("timeout")
                    {
                        eprintln!("⚠️  Network unreachable — cannot reach Amazon Braket API.");
                        eprintln!("   Falling back to local statevector simulator...");
                        let sim = SimulatorBackend::new(circuit.qubits, cfg.shots);
                        match sim.execute_circuit(&circuit).await {
                            Ok(r) => {
                                println!("\n[local sim] Shots run : {}", r.shots);
                                println!("[local sim] Runtime   : {} ms", r.execution_time_ms);
                                println!("\nCounts:");
                                let mut counts: Vec<_> = r.counts.iter().collect();
                                counts.sort_by(|a, b| b.1.cmp(a.1));
                                for (bitstring, count) in &counts {
                                    let pct = (**count as f64 / r.shots as f64) * 100.0;
                                    println!("  |{bitstring}> : {count:5}  ({pct:.1}%)", count = **count);
                                }
                            }
                            Err(se) => eprintln!("Simulator error: {se}"),
                        }
                    } else {
                        eprintln!("❌  Error running Braket task: {e}");
                        std::process::exit(1);
                    }
                }
            }
            Ok(())
        }

        QuantumCommands::RunSimulator { shots } => {
            println!(
                "Running Bell-state circuit on local statevector simulator..."
            );
            println!("  Shots : {shots}");
            let backend = SimulatorBackend::new(2, shots);
            let circuit = bell_circuit();
            let result = backend.execute_circuit(&circuit).await?;
            println!("\nShots run : {}", result.shots);
            println!("Runtime   : {} ms", result.execution_time_ms);
            println!("\nCounts:");
            let mut counts: Vec<_> = result.counts.iter().collect();
            counts.sort_by(|a, b| b.1.cmp(a.1));
            for (bitstring, count) in &counts {
                let n = **count;
                let pct = (n as f64 / result.shots as f64) * 100.0;
                println!("  |{bitstring}> : {n:5}  ({pct:.1}%)");
            }
            Ok(())
        }

        QuantumCommands::DeviceInfo { device, bucket } => {
            println!("Querying Braket device info...");
            let cfg = QuantumConfig {
                backend: "braket".to_string(),
                braket_device_arn: device.clone(),
                braket_s3_bucket: bucket,
                ..QuantumConfig::default()
            };
            let backend = AmazonBraketBackend::from_config(&cfg).await?;
            let (online, status_str) = match backend.get_device_status().await {
                Ok((o, s)) => (o, s),
                Err(e) => {
                    let msg = format!("{e:?}");
                    if msg.contains("dns")
                        || msg.contains("Connect")
                        || msg.contains("io error")
                    {
                        eprintln!("  ⚠️  No network — showing cached catalog info");
                        (false, "UNKNOWN (offline)".to_string())
                    } else if msg.contains("AccessDeniedException")
                        || msg.contains("not authorized")
                    {
                        eprintln!(
                            "  ⚠️  Braket not yet activated — showing catalog info"
                        );
                        (false, "PENDING_ACTIVATION".to_string())
                    } else {
                        return Err(e);
                    }
                }
            };
            let info = backend.get_backend_info().await;
            println!("  ID          : {}", device);
            println!("  Max qubits  : {}", info.max_qubits);
            println!("  Max shots   : {}", info.max_shots);
            println!("  Status      : {}", status_str);
            println!("  Online      : {}", online);
            println!("  Gates       : {}", info.supported_gates.join(", "));
            if let Some(ref cat) = backend.device_catalog {
                println!("  Provider    : {}", cat.provider);
                println!("  Cost/task   : ${:.4}", cat.cost_per_task_usd);
                println!("  Cost/shot   : ${:.6}", cat.cost_per_shot_usd);
            }
            Ok(())
        }

        QuantumCommands::Devices => {
            use crate::quantum::BraketDeviceCatalog;
            println!("Known Amazon Braket Devices:\n");
            println!(
                "{:<22} {:<10} {:<6} {:<10} {:<10} {}",
                "Name", "Provider", "Qubits", "Type", "$/task", "ARN"
            );
            println!("{}", "─".repeat(100));
            for d in BraketDeviceCatalog::all_devices() {
                let dtype = format!("{:?}", d.device_type);
                println!(
                    "{:<22} {:<10} {:<6} {:<10} ${:<9.4} {}",
                    d.name,
                    d.provider,
                    d.max_qubits,
                    dtype,
                    d.cost_per_task_usd,
                    d.arn
                );
            }
            Ok(())
        }

        QuantumCommands::EstimateCost {
            device,
            shots,
            circuits,
        } => {
            use crate::quantum::BraketDeviceCatalog;
            if let Some(cat) = BraketDeviceCatalog::find_by_arn(&device) {
                let per_task = cat.estimate_cost(shots);
                let total = per_task * circuits as f64;
                println!("Cost Estimate for {}:", cat.name);
                println!("  Shots/task  : {shots}");
                println!("  Circuits    : {circuits}");
                println!("  Cost/task   : ${per_task:.4}");
                println!("  Total cost  : ${total:.4}");
            } else {
                println!("Unknown device: {device}");
                println!("Run `housaky quantum devices` to list known devices.");
            }
            Ok(())
        }

        QuantumCommands::Transpile { device, opt_level } => {
            use crate::quantum::transpiler::{
                CircuitTranspiler, TranspilerConfig,
            };
            println!("Transpiling Bell circuit for target device...");
            println!("  Target : {device}");
            println!("  Opt    : level {opt_level}\n");

            let transpiler = CircuitTranspiler::new(TranspilerConfig {
                optimization_level: opt_level,
                target_device: Some(device),
                ..Default::default()
            });

            let circuit = bell_circuit();
            let (_result, report) = transpiler.transpile(&circuit)?;

            println!("Transpilation Report:");
            println!("  Target device    : {}", report.target_device);
            println!("  Native gates     : {}", report.native_gate_set.join(", "));
            println!("  Original gates   : {}", report.original_gates);
            println!("  Transpiled gates : {}", report.transpiled_gates);
            println!("  Original depth   : {}", report.original_depth);
            println!("  Transpiled depth : {}", report.transpiled_depth);
            println!("  Gates removed    : {}", report.gates_removed);
            println!("  Gates decomposed : {}", report.gates_decomposed);
            println!("  Rotations merged : {}", report.rotations_merged);
            println!("\nPasses applied:");
            for pass in &report.passes_applied {
                println!("  • {pass}");
            }
            if report.passes_applied.is_empty() {
                println!("  (none — circuit already native)");
            }
            Ok(())
        }

        QuantumCommands::Tomography { shots, qubits } => {
            use crate::quantum::tomography::{
                StateTomographer, TomographyConfig,
            };
            println!("Running quantum state tomography...");
            println!("  Qubits          : {qubits}");
            println!("  Shots per basis : {shots}\n");

            let backend = std::sync::Arc::new(SimulatorBackend::new(qubits, shots));
            let tomographer = StateTomographer::new(
                backend,
                TomographyConfig {
                    shots_per_basis: shots,
                    max_qubits: qubits,
                    ..Default::default()
                },
            );

            let mut circuit = QuantumCircuit::new(qubits);
            circuit.add_gate(Gate::h(0));
            if qubits >= 2 {
                circuit.add_gate(Gate::cnot(0, 1));
            }

            let result = tomographer.tomograph(&circuit).await?;
            println!("Tomography Results:");
            println!("  Qubits measured   : {}", result.n_qubits);
            println!("  Bases measured    : {}", result.bases_measured.join(", "));
            println!("  Total shots       : {}", result.total_shots);
            println!("  Purity            : {:.6}", result.purity);
            println!("  Trace             : {:.6}", result.trace);
            println!("  Von Neumann entropy : {:.6}", result.von_neumann_entropy);
            println!("  Valid state       : {}", result.is_valid_state);
            println!("  Runtime           : {} ms", result.runtime_ms);
            Ok(())
        }

        QuantumCommands::AgiBridge { goals } => {
            handle_agi_bridge(config, goals).await
        }

        QuantumCommands::Tasks {
            device,
            bucket,
            max,
        } => {
            println!("Listing recent Braket tasks for {device}...\n");
            let cfg = QuantumConfig {
                backend: "braket".to_string(),
                braket_device_arn: device.clone(),
                braket_s3_bucket: bucket,
                ..QuantumConfig::default()
            };
            let backend = AmazonBraketBackend::from_config(&cfg).await?;
            match backend.list_recent_tasks(max).await {
                Ok(tasks) => {
                    if tasks.is_empty() {
                        println!("  No tasks found.");
                    } else {
                        println!(
                            "{:<50} {:<12} {:<8} {}",
                            "Task ARN", "Status", "Shots", "Created"
                        );
                        println!("{}", "─".repeat(100));
                        for t in &tasks {
                            println!(
                                "{:<50} {:<12} {:<8} {}",
                                t.task_arn, t.status, t.shots, t.created_at
                            );
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("{e:?}");
                    if msg.contains("dns")
                        || msg.contains("Connect")
                        || msg.contains("io error")
                    {
                        println!("  ⚠️  No network connection to AWS — cannot list tasks.");
                        println!(
                            "  Tasks are recorded in S3 once Braket is activated."
                        );
                    } else if msg.contains("AccessDeniedException")
                        || msg.contains("not authorized")
                    {
                        println!("  ⚠️  Braket not yet activated on this account.");
                        println!("  Activate at: https://console.aws.amazon.com/braket/home");
                    } else {
                        return Err(e);
                    }
                }
            }
            Ok(())
        }

        QuantumCommands::Metrics => {
            handle_quantum_metrics(config).await
        }

        QuantumCommands::Benchmark { sizes } => {
            handle_quantum_benchmark(sizes)
        }
    }
}

async fn handle_agi_bridge(config: &Config, goals: usize) -> Result<()> {
    use crate::quantum::agi_bridge::{AgiBridgeConfig, QuantumAgiBridge};
    println!("Running Quantum AGI Bridge demo...\n");

    let bridge = if config.quantum.enabled && config.quantum.backend == "braket" {
        println!("  Backend : Amazon Braket — {}", config.quantum.braket_device_arn);
        println!("  Bucket  : s3://{}/{}", config.quantum.braket_s3_bucket, config.quantum.braket_s3_prefix);
        println!("  Budget  : ${:.2}/cycle\n", config.quantum.agi.cycle_budget_usd);
        let qcfg = QuantumConfig {
                enabled: config.quantum.enabled,
                backend: config.quantum.backend.clone(),
                braket_device_arn: config.quantum.braket_device_arn.clone(),
                braket_s3_bucket: config.quantum.braket_s3_bucket.clone(),
                braket_s3_prefix: config.quantum.braket_s3_prefix.clone(),
                max_qubits: config.quantum.max_qubits,
                shots: config.quantum.shots,
                error_mitigation: config.quantum.error_mitigation,
                ..QuantumConfig::default()
            };
        match QuantumAgiBridge::from_config(&qcfg).await {
            Ok(b) => b,
            Err(e) => {
                let msg = format!("{e:?}");
                if msg.contains("dispatch failure") || msg.contains("ConnectError") || msg.contains("dns error") || msg.contains("timeout") {
                    eprintln!("  ⚠️  Braket unreachable ({e}) — running with local simulator");
                } else {
                    eprintln!("  ⚠️  Braket init failed ({e}) — running with local simulator");
                }
                QuantumAgiBridge::new(AgiBridgeConfig {
                    quantum_threshold: 2,
                    max_qubits: config.quantum.max_qubits,
                    cycle_budget_usd: config.quantum.agi.cycle_budget_usd,
                    ..Default::default()
                })
            }
        }
    } else {
        println!("  Backend : local simulator (set quantum.enabled=true to use Braket)\n");
        QuantumAgiBridge::new(AgiBridgeConfig {
            quantum_threshold: 2,
            max_qubits: config.quantum.max_qubits,
            ..Default::default()
        })
    };

    // Demo: Goal scheduling
    let goal_ids: Vec<String> =
        (0..goals).map(|i| format!("goal_{i}")).collect();
    let mut priorities = std::collections::HashMap::new();
    for (i, id) in goal_ids.iter().enumerate() {
        priorities.insert(id.clone(), 1.0 - (i as f64 / goals as f64));
    }
    let deps = if goals >= 3 {
        vec![(goal_ids[2].clone(), goal_ids[0].clone())]
    } else {
        vec![]
    };

    let sched =
        bridge.schedule_goals(&goal_ids, &priorities, &deps).await?;
    println!("Goal Scheduling:");
    println!("  Strategy  : {}", sched.strategy);
    println!("  Objective : {:.4}", sched.objective_value);
    println!("  Advantage : {:.2}x", sched.quantum_advantage);
    println!("  Schedule  : {}", sched.schedule.join(" → "));
    println!("  Runtime   : {} ms\n", sched.runtime_ms);

    // Demo: Memory graph optimization
    let nodes: Vec<String> =
        (0..goals).map(|i| format!("node_{i}")).collect();
    let edges: Vec<(String, String, f64)> = (0..goals.saturating_sub(1))
        .map(|i| {
            (
                nodes[i].clone(),
                nodes[i + 1].clone(),
                0.5 + (i as f64 * 0.1),
            )
        })
        .collect();

    let mem = bridge.optimize_memory_graph(&nodes, &edges).await?;
    println!("Memory Graph Optimization:");
    println!("  Strategy        : {}", mem.strategy);
    println!("  Energy          : {:.4}", mem.energy);
    println!("  Strengthen      : {} edges", mem.strengthen_edges.len());
    println!("  Prune           : {} edges", mem.prune_edges.len());
    println!("  Runtime         : {} ms\n", mem.runtime_ms);

    let metrics = bridge.metrics().await;
    println!("Bridge Metrics:");
    println!("  Quantum calls     : {}", metrics.total_quantum_calls);
    println!(
        "  Classical fallback : {}",
        metrics.total_classical_fallbacks
    );
    println!(
        "  Avg advantage     : {:.2}x",
        metrics.average_quantum_advantage
    );
    Ok(())
}

async fn handle_quantum_metrics(config: &Config) -> Result<()> {
    use crate::quantum::agi_bridge::{AgiBridgeConfig, QuantumAgiBridge};
    println!("Quantum AGI Bridge Metrics\n");

    if !config.quantum.enabled {
        println!("  Status  : disabled");
        println!("  Enable  : set [quantum] enabled = true in config.toml");
        return Ok(());
    }

    let bridge_cfg = AgiBridgeConfig {
        max_qubits: config.quantum.max_qubits,
        error_mitigation: config.quantum.error_mitigation,
        transpile: config.quantum.transpile,
        target_device: if config.quantum.backend == "braket" {
            Some(config.quantum.braket_device_arn.clone())
        } else {
            None
        },
        quantum_threshold: config.quantum.agi.quantum_threshold,
        cycle_budget_usd: config.quantum.agi.cycle_budget_usd,
        goal_scheduling_shots: config.quantum.shots,
        reasoning_search_shots: config.quantum.shots * 2,
        memory_optimization_shots: config.quantum.shots / 2,
        fitness_eval_shots: config.quantum.shots * 2,
    };
    let bridge = QuantumAgiBridge::new(bridge_cfg);
    let m = bridge.metrics().await;

    println!("  Backend              : {}", config.quantum.backend);
    println!("  Max qubits           : {}", config.quantum.max_qubits);
    println!(
        "  Quantum threshold    : {} goals/nodes",
        config.quantum.agi.quantum_threshold
    );
    println!(
        "  Cycle budget         : ${:.4}",
        config.quantum.agi.cycle_budget_usd
    );
    println!();
    println!("  Total quantum calls  : {}", m.total_quantum_calls);
    println!("  Classical fallbacks  : {}", m.total_classical_fallbacks);
    println!(
        "  Avg quantum advantage: {:.2}x",
        m.average_quantum_advantage
    );
    println!("  Total cost (session) : ${:.6}", m.total_cost_usd);
    println!();
    println!("  Features enabled:");
    println!(
        "    Goal scheduling (QAOA)   : {}",
        config.quantum.agi.enable_goal_scheduling
    );
    println!(
        "    Memory optimisation (QA)  : {}",
        config.quantum.agi.enable_memory_optimization
    );
    println!(
        "    Reasoning search (Grover) : {}",
        config.quantum.agi.enable_reasoning_search
    );
    println!(
        "    Fitness exploration (VQE) : {}",
        config.quantum.agi.enable_fitness_exploration
    );
    println!(
        "    Hybrid planning (MCTS+Q)  : {}",
        config.quantum.agi.enable_planning
    );
    Ok(())
}

fn handle_quantum_benchmark(sizes: String) -> Result<()> {
    use crate::housaky::quantum::benchmarks::QuantumBenchmarkSuite;
    let problem_sizes: Vec<usize> = sizes
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    println!("Running quantum advantage benchmarks...");
    println!("  Problem sizes: {:?}\n", problem_sizes);

    let suite = QuantumBenchmarkSuite {
        solver: crate::housaky::quantum::HybridSolver::new(),
        problem_sizes,
    };
    let report = suite.run_full_suite();

    println!("Benchmark Report:");
    println!("  Total problems   : {}", report.total_problems);
    println!(
        "  Quantum advantage: {}/{}",
        report.quantum_advantaged, report.total_problems
    );
    println!("  Avg speedup      : {:.2}x", report.average_speedup);
    println!(
        "  Avg quality      : {:.2}x",
        report.average_quality_improvement
    );
    println!("  Best domain      : {}", report.best_quantum_domain);
    println!("\nDetailed Results:");
    println!(
        "{:<15} {:<6} {:<10} {:<10} {:<10} {}",
        "Type", "Size", "Classical", "Quantum", "Speedup", "Advantage"
    );
    println!("{}", "─".repeat(70));
    for r in &report.results {
        println!(
            "{:<15} {:<6} {:<10} {:<10} {:<10.2} {}",
            r.problem_type,
            r.problem_size,
            format!("{}ms", r.classical_ms),
            format!("{}ms", r.quantum_ms),
            r.speedup_ratio,
            if r.quantum_advantage { "✓" } else { "✗" }
        );
    }
    Ok(())
}
