fn main() {
    if let Err(error) = dokassist_lib::agent_trajectory_benchmark_main() {
        eprintln!("agent-trajectory-benchmark: {error}");
        std::process::exit(2);
    }
}
