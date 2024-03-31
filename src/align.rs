use rayon::prelude::*;
use std::{collections::HashMap, time::SystemTime};

use crate::fingerprint::Fingerprint;

#[derive(Clone, Copy)]
pub struct FingerprintDifference {
    pub most_common_offset: isize,
    pub most_common_offset_occurences: usize,
    pub first_sample_offset_match: usize,
}

pub fn align_fingerprints(
    source: &[Fingerprint],
    sample: &[Fingerprint],
) -> Option<FingerprintDifference> {
    let start = SystemTime::now();
    let sample_hashmap: HashMap<String, usize> = sample
        .par_iter()
        .map(|v| (v.hash.to_owned(), v.time))
        .collect::<HashMap<_, _>>();

    let matches: Vec<_> = source
        .par_iter()
        .filter(|f1| sample_hashmap.contains_key(&f1.hash))
        .map(|f1| {
            let sample_offset = *sample_hashmap
                .get(&f1.hash)
                .expect("Found a fingerprint but then it disappeared?")
                as isize;

            let offset_diff: isize = f1.time as isize - sample_offset;
            (f1.hash.clone(), f1.time, offset_diff, sample_offset)
        })
        .collect();

    let mut offset_count: HashMap<isize, usize> = HashMap::new();
    matches.iter().for_each(|m| {
        offset_count.insert(m.2, offset_count.get(&m.2).unwrap_or(&0) + 1);
    });

    let mut sorted_matches = offset_count
        .par_iter()
        .map(|v| (*v.0, *v.1))
        .collect::<Vec<(isize, usize)>>();
    sorted_matches.sort_by(|a, b| b.1.cmp(&a.1));

    let max_offset = *sorted_matches
        .first()
        .expect("Failed to get at least one match");

    let first_fingerprint: usize = matches
        .iter()
        .find(|m| m.2 == max_offset.0)
        .expect("Failed to find at least one match")
        .3 as usize;

    let end = SystemTime::now();
    println!(
        "get_offset ({:?}ms)",
        end.duration_since(start).unwrap().as_millis()
    );

    Some(FingerprintDifference {
        most_common_offset: max_offset.0,
        most_common_offset_occurences: max_offset.1,
        first_sample_offset_match: first_fingerprint,
    })
}
