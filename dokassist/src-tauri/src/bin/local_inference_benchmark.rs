fn main() {
    if let Err(error) = dokassist_lib::local_inference_benchmark_main() {
        eprintln!("local-inference-benchmark: {error}");
        std::process::exit(2);
    }
}
