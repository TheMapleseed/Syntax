use std::sync::mpsc;
use std::thread;

use crate::pipeline::{encode_packet, PipelineError};

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub workers: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            workers: 4,
        }
    }
}

#[derive(Debug)]
pub struct ConcurrentPipeline {
    config: PipelineConfig,
}

impl ConcurrentPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    pub fn encode_all<I, S>(&self, inputs: I) -> Vec<Result<String, PipelineError>>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let collected: Vec<String> = inputs.into_iter().map(Into::into).collect();
        let total = collected.len();
        if total == 0 {
            return Vec::new();
        }

        let worker_count = self.config.workers.max(1).min(total);
        let (job_tx, job_rx) = mpsc::channel::<(usize, String)>();
        let (result_tx, result_rx) = mpsc::channel::<(usize, Result<String, PipelineError>)>();

        let mut handles = Vec::with_capacity(worker_count);
        let mut worker_senders = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let (worker_tx, worker_rx) = mpsc::channel::<(usize, String)>();
            worker_senders.push(worker_tx);

            let result_tx = result_tx.clone();
            let handle = thread::spawn(move || {
                while let Ok((idx, input)) = worker_rx.recv() {
                    let result = encode_packet(&input);
                    if result_tx.send((idx, result)).is_err() {
                        break;
                    }
                }
            });
            handles.push(handle);
        }

        let distributor = {
            let worker_senders = worker_senders;
            thread::spawn(move || {
                let mut cursor = 0usize;
                while let Ok(job) = job_rx.recv() {
                    let target = cursor % worker_senders.len();
                    if worker_senders[target].send(job).is_err() {
                        break;
                    }
                    cursor += 1;
                }
            })
        };

        for (idx, input) in collected.into_iter().enumerate() {
            let _ = job_tx.send((idx, input));
        }
        drop(job_tx);
        drop(result_tx);

        let mut ordered: Vec<Option<Result<String, PipelineError>>> =
            std::iter::repeat_with(|| None).take(total).collect();

        let mut received = 0usize;
        while received < total {
            match result_rx.recv() {
                Ok((idx, res)) => {
                    if ordered[idx].is_none() {
                        ordered[idx] = Some(res);
                        received += 1;
                    }
                }
                Err(_) => break,
            }
        }

        let _ = distributor.join();
        for handle in handles {
            let _ = handle.join();
        }

        ordered
            .into_iter()
            .map(|entry| {
                entry.unwrap_or(Err(PipelineError::Validation(
                    crate::validation::ValidationError::EmptyInput,
                )))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::decode_packet;

    #[test]
    fn preserves_order_across_workers() {
        let pipeline = ConcurrentPipeline::new(PipelineConfig {
            workers: 3,
        });

        let input = vec!["first value", "second value", "third value", "fourth value"];
        let output = pipeline.encode_all(input.clone());

        assert_eq!(output.len(), input.len());
        for (idx, encoded) in output.into_iter().enumerate() {
            let encoded = encoded.unwrap();
            let decoded = decode_packet(&encoded).unwrap();
            assert_eq!(decoded, input[idx]);
        }
    }

    #[test]
    fn default_pipeline_uses_native_security_mode() {
        let pipeline = ConcurrentPipeline::new(PipelineConfig::default());
        let output = pipeline.encode_all(vec!["first value", "second value"]);
        assert_eq!(output.len(), 2);

        let one = decode_packet(&output[0].as_ref().unwrap().clone()).unwrap();
        let two = decode_packet(&output[1].as_ref().unwrap().clone()).unwrap();
        assert_eq!(one, "first value");
        assert_eq!(two, "second value");
    }
}
