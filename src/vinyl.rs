use std::{collections::HashMap, iter::zip};

use ethers::{
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{BlockNumber, Filter, Log, H256},
};
use log::{debug, error, info};

pub trait EventHandler {
    fn handle(&self, raw_log: &Log);
}

pub struct VinylSync {
    provider: Provider<Ws>,
    handlers: HashMap<String, Box<dyn EventHandler>>,
}

impl VinylSync {
    pub fn new(provider: &Provider<Ws>) -> Self {
        VinylSync {
            provider: provider.clone(),
            handlers: HashMap::new(),
        }
    }

    pub fn add_handler(&mut self, event_signature: &str, handler: Box<dyn EventHandler>) {
        self.handlers.insert(event_signature.to_string(), handler);
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let last_block = self
            .provider
            .get_block(BlockNumber::Latest)
            .await?
            .unwrap()
            .number
            .unwrap();
        info!("Last block number: {:?}", last_block);

        let mut blocks_history: Vec<H256> = vec![];
        let mut block_to_number: HashMap<H256, usize> = HashMap::new();

        let mut stream = self.provider.subscribe_blocks().await?;
        while let Some(block) = stream.next().await {
            let block_hash = block.hash.unwrap();
            let block_number = block.number.unwrap().as_usize();

            if blocks_history.is_empty() {
                blocks_history.push(block_hash);
                block_to_number.insert(block_hash, block_number);
                info!("First block: {:?}", block_hash);

                self.process_block(&block_hash).await?;
                continue;
            }

            if block.parent_hash == *blocks_history.last().unwrap() {
                blocks_history.push(block_hash);
                block_to_number.insert(block_hash, block_number);
                self.process_block(&block_hash).await?;
                debug!(
                    "Normal {:?} {:?} {:?}",
                    block.number.unwrap(),
                    block.hash,
                    block.parent_hash
                );
                continue;
            }

            // Reorg detected, we need to rollback
            let mut parent_hash = block.parent_hash;
            let mut parent_number = block.number.unwrap().as_usize() - 1;
            let mut new_blocks: Vec<H256> = vec![block_hash];
            let mut new_block_numbers: Vec<usize> = vec![block.number.unwrap().as_usize()];

            error!("Reorg detected for {:?}, rolling back", block_hash);

            // First - find common ancestor
            while !block_to_number.contains_key(&parent_hash) {
                info!("Fetching parent of {:?}", parent_hash);
                new_blocks.push(parent_hash);
                new_block_numbers.push(parent_number);
                if new_blocks.len() > blocks_history.len() {
                    panic!("New blocks are more than history");
                }

                let parent_block = self.provider.get_block(parent_hash).await?.unwrap();
                parent_hash = parent_block.parent_hash;
                parent_number = parent_block.number.unwrap().as_usize() - 1;
                info!("Fetched value {:?}", parent_hash);
            }
            new_blocks.reverse();
            new_block_numbers.reverse();

            // Second drop all known blocks after common ancestor
            while let Some(block_hash) = blocks_history.pop() {
                if block_hash == parent_hash {
                    break;
                }
                let dropped_block_number = block_to_number[&block_hash];
                info!("Dropping block {:?} {:?}", dropped_block_number, block_hash);
                block_to_number.remove(&block_hash);
            }

            info!(
                "Appending new blocks: {:?} {:?}",
                new_blocks, new_block_numbers
            );
            info!(
                "Verifying sequence. Parent: {:?} Parent number: {:?} New block number: {:?}",
                parent_hash, block_to_number[&parent_hash], new_block_numbers[0]
            );
            assert!(block_to_number[&parent_hash] + 1 == new_block_numbers[0]);

            for (block_number, block_hash) in zip(new_block_numbers.iter(), new_blocks.iter()) {
                block_to_number.insert(*block_hash, *block_number);

                self.process_block(block_hash).await?;
            }
            blocks_history.append(&mut new_blocks);

            info!(
                "Fixed revert {:?} {:?} {:?}",
                block.number.unwrap(),
                block.hash,
                block.parent_hash
            );
        }

        return Ok(());
    }

    async fn process_block(&self, block_hash: &H256) -> Result<(), Box<dyn std::error::Error>> {
        for (event_signature, handler) in self.handlers.iter() {
            self.process_block_for_event(event_signature, handler, block_hash)
                .await?;
        }

        Ok(())
    }

    async fn process_block_for_event(
        &self,
        event_signature: &String,
        handler: &Box<dyn EventHandler>,
        block_hash: &H256,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let filter = Filter::new()
            .at_block_hash(*block_hash)
            .event(event_signature.as_str());

        let logs = self.provider.get_logs(&filter).await.map_err(|e| {
            error!("Failed to fetch logs: {:?}", e);
            e
        })?;

        if logs.is_empty() {
            debug!("No logs found");
        } else {
            debug!("Found {:?} logs", logs.len());
        }

        for log in logs {
            handler.handle(&log);
        }

        Ok(())
    }
}
