/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

/// Peer Quality Metrics
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::thread;
use std::time;
use sysinfo::{NetworkExt, ProcessExt, System, SystemExt};

// peer metric constants
const CPU_STRESS_WEIGHT: f64 = 2_f64;
const NETWORK_STRESS_WEIGHT: f64 = 0.001_f64;
const DISK_STRESS_WEIGHT: f64 = 0.001_f64;

lazy_static! {
    pub static ref PEER_METRICS: Mutex<PeerMetrics> = Mutex::new(PeerMetrics::new());
}

#[derive(Default)]
pub struct PeerMetrics {
    system: System,
}

impl PeerMetrics {
    pub fn new() -> Self {
        let mut peer_metrics = Self {
            system: System::new_all(),
        };
        peer_metrics.initialize();
        peer_metrics
    }

    fn initialize(&mut self) {
        self.system.refresh_all();
        thread::sleep(time::Duration::from_millis(500));
        self.system.refresh_all();
    }

    /// Get the local stress metric to advertise to peers
    pub fn get_quality_metric(&mut self) -> f64 {
        let mut qm = get_cpu_stress(&mut self.system) * CPU_STRESS_WEIGHT;
        qm += get_network_stress(&mut self.system) * NETWORK_STRESS_WEIGHT;
        qm + get_disk_stress(&mut self.system) * DISK_STRESS_WEIGHT
    }
}

// This function gets the current CPU load on the system.
fn get_cpu_stress(system: &mut System) -> f64 {
    system.refresh_all();

    let load_avg = system.load_average();
    load_avg.one //using the average over the last 1 minute
}

// This function gets the current network load on the system
fn get_network_stress(system: &mut System) -> f64 {
    system.refresh_all();

    let networks = system.networks();

    let mut packets_in = 0;
    let mut packets_out = 0;
    for (_interface_name, network) in networks {
        packets_in += network.received();
        packets_out += network.transmitted();
    }
    (packets_in as f64) + (packets_out as f64)
    //TODO: add network card capabilities to the metric. cards with > network capacity should get a lower stress number.
}

fn get_disk_stress(system: &mut System) -> f64 {
    system.refresh_all();

    // Sum up the disk usage measured as total read and writes per process:
    let mut total_usage = 0_u64;
    for process in system.processes().values() {
        let usage = process.disk_usage();
        total_usage = total_usage + usage.total_written_bytes + usage.total_read_bytes;
    }
    total_usage as f64
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use std::fs::{remove_file, OpenOptions};
    use std::io::Write;
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    const CPU_THREADS: usize = 200;
    const NETWORK_THREADS: usize = 10;

    #[test]
    fn cpu_load_test() {
        let mut peer_metrics = PeerMetrics::new();

        let loading = Arc::new(AtomicBool::new(true));

        //first measure of CPU for benchmark
        let qm1 = get_cpu_stress(&mut peer_metrics.system) * CPU_STRESS_WEIGHT;

        //set CPU on fire to measure stress
        let mut threads = vec![];
        for _i in 0..CPU_THREADS {
            threads.push(thread::spawn({
                let mut cpu_fire = 0;
                let loading_test = loading.clone();
                move || {
                    while loading_test.load(Ordering::Relaxed) {
                        cpu_fire += 1;
                        if cpu_fire % 1_000_000_000 == 0 {
                            println!("Got to 1 billion...");
                        }
                    }
                }
            }));
        }

        thread::sleep(Duration::from_millis(10000)); //let cpu spin up

        //second measure of CPU
        let qm2 = get_cpu_stress(&mut peer_metrics.system) * CPU_STRESS_WEIGHT;
        println!("cpu: QM1: {}, QM2: {}", qm1, qm2);
        assert!(qm2 > qm1);
        loading.store(false, Ordering::Relaxed); //kill threads

        //wait for threads
        for thread in threads {
            thread.join().unwrap();
        }
        //we could add another measure of CPU did no think it was that important
    }

    #[test]
    fn network_load_test() {
        let mut peer_metrics = PeerMetrics::new();

        let loading = Arc::new(AtomicBool::new(true));

        //fist measure of network for benchmark
        let qm1 = get_network_stress(&mut peer_metrics.system) * NETWORK_STRESS_WEIGHT;

        //shotgun the network with packets
        let mut threads = vec![];
        for i in 0..NETWORK_THREADS {
            threads.push(thread::spawn({
                let address: String = format_args!("127.0.0.1:3425{i}").to_string();
                let socket = UdpSocket::bind(address).expect("couldn't bind to address");
                let loading_test = loading.clone();
                move || {
                    while loading_test.load(Ordering::Relaxed) {
                        socket
                            .send_to(&[0; 10], "127.0.0.1:4242")
                            .expect("couldn't send data");
                    }
                }
            }));
        }

        thread::sleep(Duration::from_millis(5000)); //let network traffic happen

        let qm2 = get_network_stress(&mut peer_metrics.system) * NETWORK_STRESS_WEIGHT;
        println!("network: QM1: {}, QM2: {}", qm1, qm2);
        assert!(qm2 > qm1);
        loading.store(false, Ordering::Relaxed); //kill threads

        //wait for threads
        for thread in threads {
            thread.join().unwrap();
        }
        //we could add another measure of network did no think it was that important
    }

    #[test]
    fn disk_load_test() {
        let mut peer_metrics = PeerMetrics::new();

        let loading = Arc::new(AtomicBool::new(true));
        let test_file = "pyrsia_test.txt";

        // fist measure of network for benchmark
        let qm1 = get_disk_stress(&mut peer_metrics.system) * DISK_STRESS_WEIGHT;

        // write some data
        let write_thread = thread::spawn({
            let file_data = "Some test data for the file!\n";
            let except_str = format!("Unable to open file {}", test_file);
            let mut f = OpenOptions::new()
                .append(true)
                .create(true)
                .open(test_file)
                .expect(&except_str);
            let loading_test = loading.clone();
            move || {
                while loading_test.load(Ordering::Relaxed) {
                    f.write_all(file_data.as_bytes())
                        .expect("Unable to write data");
                }
                drop(f);
            }
        });

        thread::sleep(Duration::from_millis(500)); //let writes happen

        // second measure of network
        let qm2 = get_disk_stress(&mut peer_metrics.system) * DISK_STRESS_WEIGHT;
        loading.store(false, Ordering::Relaxed); //kill thread
        write_thread.join().unwrap();
        remove_file(test_file).unwrap();
        println!("disk: QM1: {}, QM2: {}", qm1, qm2);
        assert!(qm2 > qm1);

        //we could add another measure of disks did no think it was that important
    }

    #[test]
    fn quality_metric_test() {
        let mut peer_metrics = PeerMetrics::new();

        let quality_metric = peer_metrics.get_quality_metric();
        assert!(quality_metric != 0_f64);
    }
}
