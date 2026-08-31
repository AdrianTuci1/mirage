#![cfg(feature = "onnx")]
//! End-to-end proof that text and images really share one embedding space.
//!
//! This is the only test that exercises the actual CLIP weights, so it is skipped
//! unless both the models and the photographs are on disk:
//!
//! ```sh
//! ../../scripts/fetch-clip-models.sh test-models
//! ../../scripts/fetch-test-images.py test-images
//! MIRAGE_CLIP_MODELS=test-models cargo test --test clip_space
//! ```

use mirage_daemon::db::cosine_similarity;
use mirage_daemon::embeddings::{find_clip_artifacts, ClipEmbedder, Embedder};
use mirage_daemon::models::{Record, RecordWithScore};
use mirage_daemon::search::interleave_modalities;
use std::path::{Path, PathBuf};

/// Canonical CLIP prompt templates: `a photo of a {label}` is how the model was
/// trained to read a caption, so the test queries use that shape.
const CASES: &[(&str, &str)] = &[
    ("cat.jpg", "a photo of a cat"),
    ("beach.jpg", "a photo of a sandy beach"),
    ("snow.jpg", "a photo of a snowy mountain"),
    ("car.jpg", "a photo of a car"),
    ("bridge.jpg", "a photo of a bridge"),
    ("city.jpg", "a photo of a city skyline at night"),
];

fn setup() -> Option<(ClipEmbedder, PathBuf)> {
    let models = std::env::var("MIRAGE_CLIP_MODELS")
        .ok()
        .map(PathBuf::from)?;
    let artifacts = find_clip_artifacts(&models).unwrap_or_else(|| {
        panic!(
            "no CLIP text/vision/tokenizer set under {}",
            models.display()
        )
    });
    let embedder = ClipEmbedder::new(&artifacts).expect("CLIP embedder should load");
    let images =
        PathBuf::from(std::env::var("MIRAGE_TEST_IMAGES").unwrap_or_else(|_| "test-images".into()));
    Some((embedder, images))
}

#[test]
fn both_encoders_produce_one_comparable_space() {
    let Some((embedder, _)) = setup() else {
        eprintln!("skipping: MIRAGE_CLIP_MODELS is not set");
        return;
    };
    assert!(embedder.supports_images());
    assert!(embedder.is_semantic());

    let text = embedder.embed_text("a photo of a cat").unwrap();
    let image = embedder
        .embed_image_file(&Path::new("test-images/cat.jpg"))
        .unwrap();
    assert_eq!(
        text.len(),
        image.len(),
        "text and image vectors must have one width"
    );
    assert_eq!(text.len(), embedder.dimension());
    // Cosine needs unit vectors; the encoders normalize, so a vector against itself
    // is exactly one.
    assert!((cosine_similarity(&text, &text) - 1.0).abs() < 1e-4);
    assert!((cosine_similarity(&image, &image) - 1.0).abs() < 1e-4);
    // A matching pair is far closer than an unrelated text/image pair, which is the
    // whole point of a shared space.
    let unrelated = embedder.embed_text("quarterly financial report").unwrap();
    let matched = cosine_similarity(&text, &image);
    let mismatched = cosine_similarity(&unrelated, &image);
    assert!(
        matched > mismatched + 0.05,
        "matching pair scored {matched:.3}, unrelated {mismatched:.3}"
    );
}

#[test]
fn a_text_query_finds_the_right_photograph() {
    let Some((embedder, images)) = setup() else {
        eprintln!("skipping: MIRAGE_CLIP_MODELS is not set");
        return;
    };
    let mut labelled: Vec<(&str, Vec<f32>)> = Vec::new();
    for (file, _) in CASES {
        let path = images.join(file);
        if !path.exists() {
            eprintln!("skipping: {} is missing", path.display());
            return;
        }
        labelled.push((file, embedder.embed_image_file(&path).unwrap()));
    }

    let mut wrong = Vec::new();
    for (file, query) in CASES {
        let vector = embedder.embed_text(query).unwrap();
        let mut ranked: Vec<(&str, f32)> = labelled
            .iter()
            .map(|(candidate, image)| (*candidate, cosine_similarity(&vector, image)))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        println!(
            "{query:?} -> {}",
            ranked
                .iter()
                .map(|(c, s)| format!("{c} {s:.3}"))
                .collect::<Vec<_>>()
                .join(" | ")
        );
        if &ranked[0].0 != file {
            wrong.push(format!("{query} matched {} instead of {file}", ranked[0].0));
        }
        // The correct photo must also beat the *second* one by a margin, otherwise the
        // ranking is noise.
        assert!(
            ranked[0].1 > ranked[1].1,
            "{query}: top two are tied at {:.3}",
            ranked[0].1
        );
    }
    assert!(
        wrong.is_empty(),
        "misclassified queries:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn similar_sentences_land_near_each_other() {
    let Some((embedder, _)) = setup() else {
        eprintln!("skipping: MIRAGE_CLIP_MODELS is not set");
        return;
    };
    let anchor = embedder
        .embed_text("a kitten playing with a ball of yarn")
        .unwrap();
    let related = embedder
        .embed_text("a young cat playing with yarn")
        .unwrap();
    let unrelated = embedder
        .embed_text("the invoice total was paid by wire transfer")
        .unwrap();
    assert!(
        cosine_similarity(&anchor, &related) > cosine_similarity(&anchor, &unrelated),
        "paraphrases should be closer than unrelated sentences"
    );
}

#[test]
fn a_batch_matches_the_single_item_path() {
    let Some((embedder, images)) = setup() else {
        eprintln!("skipping: MIRAGE_CLIP_MODELS is not set");
        return;
    };
    let paths: Vec<PathBuf> = CASES
        .iter()
        .map(|(file, _)| images.join(file))
        .filter(|p| p.exists())
        .collect();
    if paths.len() < 2 {
        eprintln!("skipping: fewer than two test images");
        return;
    }
    let batched = embedder.embed_image_files(&paths);
    for (path, batched) in paths.iter().zip(batched) {
        let single = embedder.embed_image_file(path).unwrap();
        let batched = batched.unwrap();
        assert_eq!(single.len(), batched.len());
        let similarity = cosine_similarity(&single, &batched);
        assert!(
            similarity > 0.98,
            "{} changed between single and batched inference: {similarity:.3}",
            path.display()
        );
    }
}

/// Documents and photographs share one LanceDB table, so they compete for the
/// same visible window. This is the before/after for that competition: the raw
/// cosine ranking is dominated by text, the interleaved ranking is not.
#[test]
fn photographs_survive_a_corpus_of_documents() {
    let Some((embedder, images)) = setup() else {
        eprintln!("skipping: MIRAGE_CLIP_MODELS is not set");
        return;
    };
    let documents: Vec<(&str, &str)> = vec![
        ("q3-report.md", "Revenue grew 12 percent quarter over quarter, driven by enterprise renewals and a 4 percent reduction in churn across the EMEA region."),
        ("standup.md", "Blocked on the staging deploy. The runner image is stale and the cache key does not include the lockfile, so every job rebuilds from source."),
        ("groceries.txt", "olive oil, sourdough, ricotta, espresso pods, lemons, thyme, sea salt, dark chocolate, sparkling water."),
        ("changelog.txt", "Fixed a crash when the window lost focus during a drag, added a shortcut for the clipboard history, dropped the legacy import path."),
        ("invoice.txt", "Invoice 2041. Net 30. Bank transfer to the account listed on the footer, reference the invoice number on the payment."),
        ("workout.txt", "Monday: five by five squat, bench and deadlift. Thursday: rows, overhead press, pull-ups. Rest ninety seconds between sets."),
    ];

    let mut pool: Vec<RecordWithScore> = Vec::new();
    for (file, _) in CASES {
        let vector = embedder.embed_image_file(&images.join(file)).unwrap();
        pool.push(hit(file, "image", vector));
    }
    let text_vectors: Vec<Vec<f32>> = documents
        .iter()
        .map(|(_, body)| embedder.embed_text(body).unwrap())
        .collect();
    for ((name, _), vector) in documents.iter().zip(text_vectors) {
        pool.push(hit(name, "text", vector));
    }

    for (file, query) in CASES {
        let vector = embedder.embed_text(query).unwrap();
        let mut ranked = pool.clone();
        for record in ranked.iter_mut() {
            record.score = cosine_similarity(&vector, &record.record.vector) as f64;
        }
        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        let raw: Vec<&str> = ranked
            .iter()
            .take(6)
            .map(|r| r.record.id.as_str())
            .collect();
        let interleaved = interleave_modalities(ranked.clone(), 6);
        let mixed: Vec<&str> = interleaved.iter().map(|r| r.record.id.as_str()).collect();
        println!("query {query:?}");
        println!("  raw cosine   {}", raw.join(" "));
        println!("  interleaved  {}", mixed.join(" "));
        let photos_before = raw.iter().filter(|id| id.ends_with(".jpg")).count();
        let photos_after = mixed.iter().filter(|id| id.ends_with(".jpg")).count();
        assert!(
            mixed.contains(file),
            "{query}: {file} missing from the interleaved window {mixed:?}"
        );
        assert!(
            photos_after > photos_before,
            "{query}: interleaving did not add photographs ({photos_before} -> {photos_after})"
        );
    }
}

fn hit(id: &str, modality: &str, vector: Vec<f32>) -> RecordWithScore {
    RecordWithScore {
        record: Record {
            id: id.to_string(),
            relative_path: id.to_string(),
            source_type: "local".to_string(),
            vector,
            updated_at: String::new(),
            version: 0,
            modality: modality.to_string(),
            caption: String::new(),
        },
        score: 0.0,
    }
}
