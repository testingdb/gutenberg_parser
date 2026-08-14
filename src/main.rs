use bzip2::read::BzDecoder;
use clap::Parser;
use crossbeam_channel::{bounded, Receiver, Sender};
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;
use roxmltree::{Document, Node};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::sync::LazyLock;
use std::time::Instant;
use tar::Archive;

// ---------------------------------------------------------------------------
// Static Configuration & Data Tables
// ---------------------------------------------------------------------------

static NAMESPACES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
    m.insert("dcterms", "http://purl.org/dc/terms/");
    m.insert("pgterms", "http://www.gutenberg.org/2009/pgterms/");
    m.insert("marcrel", "http://id.loc.gov/vocabulary/relators/");
    m.insert("cc", "http://web.resource.org/cc/");
    m.insert("rdfs", "http://www.w3.org/2000/01/rdf-schema#");
    m
});

static GUTENBERG_MIRRORS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("gutenberg", "https://www.gutenberg.org/");
    m.insert("pglaf", "https://gutenberg.pglaf.org/");
    m.insert("odu", "https://mirror.cs.odu.edu/gutenberg/");
    m.insert("waterloo", "http://mirror.csclub.uwaterloo.ca/gutenberg/");
    m.insert(
        "uk",
        "http://www.mirrorservice.org/sites/ftp.ibiblio.org/pub/docs/books/gutenberg/",
    );
    m.insert("xmission", "http://mirrors.xmission.com/gutenberg/");
    m
});

static LC_MAP: LazyLock<HashMap<&'static str, (&'static str, &'static str)>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("AC", ("Periodicals & Reference", "General Collections"));
    m.insert("AE", ("Periodicals & Reference", "Encyclopedias"));
    m.insert("AG", ("Periodicals & Reference", "Dictionaries & Reference"));
    m.insert("AI", ("Periodicals & Reference", "Indexes"));
    m.insert("AM", ("Periodicals & Reference", "Museums & Collecting"));
    m.insert("AN", ("Periodicals & Reference", "Newspapers"));
    m.insert("AP", ("Periodicals & Reference", "Periodicals & Journals"));
    m.insert("AS", ("Periodicals & Reference", "Academia & Societies"));
    m.insert("AY", ("Periodicals & Reference", "Almanacs & Yearbooks"));
    m.insert("AZ", ("Periodicals & Reference", "History of Scholarship"));
    m.insert("B", ("Philosophy & Religion", "Philosophy"));
    m.insert("BC", ("Philosophy & Religion", "Logic"));
    m.insert("BD", ("Philosophy & Religion", "Metaphysics"));
    m.insert("BF", ("Philosophy & Religion", "Psychology"));
    m.insert("BH", ("Philosophy & Religion", "Aesthetics"));
    m.insert("BJ", ("Philosophy & Religion", "Ethics"));
    m.insert("BL", ("Philosophy & Religion", "Religion & Mythology"));
    m.insert("BM", ("Philosophy & Religion", "Judaism"));
    m.insert("BP", ("Philosophy & Religion", "Islam & Eastern Religions"));
    m.insert("BQ", ("Philosophy & Religion", "Buddhism"));
    m.insert("BR", ("Philosophy & Religion", "Christianity"));
    m.insert("BS", ("Philosophy & Religion", "Biblical Studies"));
    m.insert("BT", ("Philosophy & Religion", "Doctrinal Theology"));
    m.insert("BV", ("Philosophy & Religion", "Practical Theology"));
    m.insert("BX", ("Philosophy & Religion", "Christian Denominations"));
    m.insert("CB", ("History & Geography", "History of Civilization"));
    m.insert("CC", ("History & Geography", "Archaeology"));
    m.insert("CD", ("History & Geography", "Archives & Manuscripts"));
    m.insert("CE", ("History & Geography", "Chronology"));
    m.insert("CJ", ("History & Geography", "Numismatics"));
    m.insert("CN", ("History & Geography", "Inscriptions & Epigraphy"));
    m.insert("CR", ("History & Geography", "Heraldry"));
    m.insert("CS", ("History & Geography", "Genealogy"));
    m.insert("CT", ("History & Geography", "Biography & Memoir"));
    m.insert("D", ("History & Geography", "General World History"));
    m.insert("DA", ("History & Geography", "British History"));
    m.insert("DB", ("History & Geography", "Central European History"));
    m.insert("DC", ("History & Geography", "French History"));
    m.insert("DD", ("History & Geography", "German History"));
    m.insert("DE", ("History & Geography", "Greco-Roman History"));
    m.insert("DF", ("History & Geography", "Greek History"));
    m.insert("DG", ("History & Geography", "Italian History"));
    m.insert("DH", ("History & Geography", "Low Countries History"));
    m.insert("DJ", ("History & Geography", "Dutch History"));
    m.insert("DK", ("History & Geography", "Slavic & Russian History"));
    m.insert("DL", ("History & Geography", "Scandinavian History"));
    m.insert("DP", ("History & Geography", "Spanish & Portuguese History"));
    m.insert("DQ", ("History & Geography", "Swiss History"));
    m.insert("DR", ("History & Geography", "Balkan History"));
    m.insert("DS", ("History & Geography", "Asian History"));
    m.insert("DT", ("History & Geography", "African History"));
    m.insert("DU", ("History & Geography", "Oceania History"));
    m.insert("DX", ("History & Geography", "Romani History"));
    m.insert("E", ("History & Geography", "American History"));
    m.insert("F", ("History & Geography", "Americas Local History"));
    m.insert("G", ("History & Geography", "Geography & Exploration"));
    m.insert("GA", ("History & Geography", "Cartography"));
    m.insert("GB", ("History & Geography", "Physical Geography"));
    m.insert("GC", ("History & Geography", "Oceanography"));
    m.insert("GE", ("Science & Technology", "Environmental Sciences"));
    m.insert("GF", ("Social Sciences", "Human Ecology"));
    m.insert("GN", ("Social Sciences", "Anthropology"));
    m.insert("GR", ("Literature & Fiction", "Folklore & Mythology"));
    m.insert("GT", ("Social Sciences", "Manners & Customs"));
    m.insert("GV", ("Arts & Recreation", "Sports & Recreation"));
    m.insert("H", ("Social Sciences", "General Social Sciences"));
    m.insert("HA", ("Social Sciences", "Statistics"));
    m.insert("HB", ("Social Sciences", "Economics"));
    m.insert("HC", ("Social Sciences", "Economic History"));
    m.insert("HD", ("Social Sciences", "Industry & Labor"));
    m.insert("HE", ("Social Sciences", "Transportation & Communication"));
    m.insert("HF", ("Social Sciences", "Commerce & Business"));
    m.insert("HG", ("Social Sciences", "Finance"));
    m.insert("HJ", ("Social Sciences", "Public Finance"));
    m.insert("HM", ("Social Sciences", "Sociology"));
    m.insert("HN", ("Social Sciences", "Social History"));
    m.insert("HQ", ("Social Sciences", "Family & Gender"));
    m.insert("HS", ("Social Sciences", "Societies & Clubs"));
    m.insert("HT", ("Social Sciences", "Communities & Races"));
    m.insert("HV", ("Social Sciences", "Criminology & Social Work"));
    m.insert("HX", ("Social Sciences", "Socialism & Anarchism"));
    m.insert("J", ("Law & Government", "Political Science"));
    m.insert("JA", ("Law & Government", "Political Science"));
    m.insert("JC", ("Law & Government", "Political Theory"));
    m.insert("JF", ("Law & Government", "Public Administration"));
    m.insert("JK", ("Law & Government", "United States Government"));
    m.insert("JL", ("Law & Government", "Americas Government"));
    m.insert("JN", ("Law & Government", "European Government"));
    m.insert("JQ", ("Law & Government", "Asian & African Government"));
    m.insert("JS", ("Law & Government", "Local Government"));
    m.insert("JV", ("Law & Government", "Immigration & Colonies"));
    m.insert("JX", ("Law & Government", "International Law"));
    m.insert("JZ", ("Law & Government", "International Relations"));
    m.insert("K", ("Law & Government", "Law & Legal Studies"));
    m.insert("KD", ("Law & Government", "British Law"));
    m.insert("KF", ("Law & Government", "United States Law"));
    m.insert("L", ("Social Sciences", "Education"));
    m.insert("M", ("Arts & Recreation", "Music"));
    m.insert("N", ("Arts & Recreation", "Fine Arts"));
    m.insert("NA", ("Arts & Recreation", "Architecture"));
    m.insert("NB", ("Arts & Recreation", "Sculpture"));
    m.insert("NC", ("Arts & Recreation", "Drawing & Illustration"));
    m.insert("ND", ("Arts & Recreation", "Painting"));
    m.insert("NE", ("Arts & Recreation", "Printmaking"));
    m.insert("NK", ("Arts & Recreation", "Decorative Arts"));
    m.insert("NX", ("Arts & Recreation", "Arts & Culture"));
    m.insert("P", ("Literature & Fiction", "Linguistics"));
    m.insert("PA", ("Literature & Fiction", "Classical Literature"));
    m.insert("PB", ("Literature & Fiction", "Celtic Languages"));
    m.insert("PC", ("Literature & Fiction", "Romance Languages & Literature"));
    m.insert("PD", ("Literature & Fiction", "Germanic Languages"));
    m.insert("PE", ("Literature & Fiction", "English Language"));
    m.insert("PF", ("Literature & Fiction", "West Germanic Languages"));
    m.insert("PG", ("Literature & Fiction", "Slavic Literature"));
    m.insert("PH", ("Literature & Fiction", "Uralic & Baltic Languages"));
    m.insert("PJ", ("Literature & Fiction", "Semitic Languages & Literature"));
    m.insert("PK", ("Literature & Fiction", "Indo-Iranian Languages"));
    m.insert("PL", ("Literature & Fiction", "East Asian & African Literature"));
    m.insert("PM", ("Literature & Fiction", "Indigenous American Languages"));
    m.insert("PN", ("Literature & Fiction", "Drama & Literary History"));
    m.insert("PQ", ("Literature & Fiction", "French, Italian & Spanish Literature"));
    m.insert("PR", ("Literature & Fiction", "British Literature"));
    m.insert("PS", ("Literature & Fiction", "American Literature"));
    m.insert("PT", ("Literature & Fiction", "German & Nordic Literature"));
    m.insert("PZ", ("Literature & Fiction", "Children's & Juvenile Literature"));
    m.insert("Q", ("Science & Technology", "General Science"));
    m.insert("QA", ("Science & Technology", "Mathematics & Computing"));
    m.insert("QB", ("Science & Technology", "Astronomy"));
    m.insert("QC", ("Science & Technology", "Physics"));
    m.insert("QD", ("Science & Technology", "Chemistry"));
    m.insert("QE", ("Science & Technology", "Geology & Earth Sciences"));
    m.insert("QH", ("Science & Technology", "Biology & Natural History"));
    m.insert("QK", ("Science & Technology", "Botany"));
    m.insert("QL", ("Science & Technology", "Zoology"));
    m.insert("QM", ("Science & Technology", "Human Anatomy"));
    m.insert("QP", ("Science & Technology", "Physiology"));
    m.insert("QR", ("Science & Technology", "Microbiology"));
    m.insert("R", ("Science & Technology", "Medicine & Health"));
    m.insert("S", ("Science & Technology", "Agriculture & Forestry"));
    m.insert("T", ("Science & Technology", "Technology & Engineering"));
    m.insert("U", ("History & Geography", "Military History & Science"));
    m.insert("V", ("History & Geography", "Naval History & Science"));
    m.insert("Z", ("Periodicals & Reference", "Library Science & Bibliography"));
    m
});

static BOOKSHELF_MAP: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r"(?i)science[- ]fiction|fantasy").unwrap(),
            "Science Fiction & Fantasy",
        ),
        (Regex::new(r"(?i)horror").unwrap(), "Horror & Gothic"),
        (Regex::new(r"(?i)crime|thriller|mystery").unwrap(), "Mystery & Crime"),
        (Regex::new(r"(?i)novel").unwrap(), "Fiction & Novels"),
        (Regex::new(r"(?i)british literature").unwrap(), "British Literature"),
        (Regex::new(r"(?i)american literature").unwrap(), "American Literature"),
        (Regex::new(r"(?i)classics").unwrap(), "Classics"),
        (Regex::new(r"(?i)humo?ur").unwrap(), "Humor & Satire"),
        (Regex::new(r"(?i)play|drama|theatre?").unwrap(), "Drama & Theater"),
        (Regex::new(r"(?i)law|criminology").unwrap(), "Law & Legal Studies"),
        (Regex::new(r"(?i)history - british").unwrap(), "British History"),
        (Regex::new(r"(?i)history - american").unwrap(), "American History"),
        (Regex::new(r"(?i)history - european").unwrap(), "European History"),
        (Regex::new(r"(?i)history - medieval").unwrap(), "Medieval History"),
        (Regex::new(r"(?i)history - modern").unwrap(), "Modern History"),
        (Regex::new(r"(?i)biograph").unwrap(), "Biography & Memoir"),
        (Regex::new(r"(?i)parenthood|family").unwrap(), "Family & Relationships"),
        (
            Regex::new(r"(?i)essay|letter|speech").unwrap(),
            "Essays & Literary Collections",
        ),
        (
            Regex::new(r"(?i)religion|spirituality").unwrap(),
            "Religion & Spirituality",
        ),
        (Regex::new(r"(?i)adventure").unwrap(), "Action & Adventure"),
        (
            Regex::new(r"(?i)journalism|media|writing").unwrap(),
            "Journalism & Media",
        ),
        (Regex::new(r"(?i)poetry").unwrap(), "Poetry"),
        (Regex::new(r"(?i)children|juvenile").unwrap(), "Children's Literature"),
        (Regex::new(r"(?i)philosophy").unwrap(), "Philosophy"),
        (Regex::new(r"(?i)art|architecture").unwrap(), "Art & Architecture"),
        (Regex::new(r"(?i)music").unwrap(), "Music"),
        (Regex::new(r"(?i)politics|government").unwrap(), "Politics & Government"),
        (Regex::new(r"(?i)economics|business").unwrap(), "Economics & Business"),
        (Regex::new(r"(?i)sociology").unwrap(), "Sociology"),
        (
            Regex::new(r"(?i)science[- ]nature|natural history|natural science").unwrap(),
            "Science & Nature",
        ),
        (
            Regex::new(r"(?i)technology|engineering").unwrap(),
            "Technology & Engineering",
        ),
        (Regex::new(r"(?i)travel|exploration").unwrap(), "Travel & Exploration"),
    ]
});

static LCSH_FORM_GENRE_MAP: LazyLock<HashMap<&'static str, (&'static str, &'static str)>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("fiction", ("Literature & Fiction", "Fiction & Novels"));
    m.insert("juvenile fiction", ("Literature & Fiction", "Children's Literature"));
    m.insert("juvenile literature", ("Literature & Fiction", "Children's Literature"));
    m.insert("juvenile poetry", ("Literature & Fiction", "Children's Literature"));
    m.insert("biography", ("History & Geography", "Biography & Memoir"));
    m.insert("autobiography", ("History & Geography", "Biography & Memoir"));
    m.insert("humor", ("Humor & Entertainment", "Humor & Satire"));
    m.insert("satire", ("Humor & Entertainment", "Humor & Satire"));
    m.insert("drama", ("Literature & Fiction", "Drama & Theater"));
    m.insert("poetry", ("Literature & Fiction", "Poetry"));
    m.insert("short stories", ("Literature & Fiction", "Fiction & Novels"));
    m
});

static GENRE_TO_DOMAIN_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("Science Fiction & Fantasy", "Literature & Fiction");
    m.insert("Fiction & Novels", "Literature & Fiction");
    m.insert("British Literature", "Literature & Fiction");
    m.insert("American Literature", "Literature & Fiction");
    m.insert("Classics", "Literature & Fiction");
    m.insert("Drama & Theater", "Literature & Fiction");
    m.insert("Action & Adventure", "Literature & Fiction");
    m.insert("Horror & Gothic", "Literature & Fiction");
    m.insert("Mystery & Crime", "Literature & Fiction");
    m.insert("Poetry", "Literature & Fiction");
    m.insert("Children's Literature", "Literature & Fiction");
    m.insert("Humor & Satire", "Humor & Entertainment");
    m.insert("Essays & Literary Collections", "Literature & Fiction");
    m.insert("Classical Literature", "Literature & Fiction");
    m.insert("French, Italian & Spanish Literature", "Literature & Fiction");
    m.insert("German & Nordic Literature", "Literature & Fiction");
    m.insert("British History", "History & Geography");
    m.insert("Medieval History", "History & Geography");
    m.insert("American History", "History & Geography");
    m.insert("European History", "History & Geography");
    m.insert("Modern History", "History & Geography");
    m.insert("French History", "History & Geography");
    m.insert("German History", "History & Geography");
    m.insert("General World History", "History & Geography");
    m.insert("Biography & Memoir", "History & Geography");
    m.insert("Law & Legal Studies", "Law & Government");
    m.insert("British Law", "Law & Government");
    m.insert("United States Law", "Law & Government");
    m.insert("European Government", "Law & Government");
    m.insert("Political Science", "Law & Government");
    m.insert("Family & Relationships", "Social Sciences");
    m.insert("Religion & Spirituality", "Philosophy & Religion");
    m.insert("Christianity", "Philosophy & Religion");
    m.insert("Philosophy", "Philosophy & Religion");
    m.insert("Journalism & Media", "Periodicals & Reference");
    m.insert("Periodicals & Journals", "Periodicals & Reference");
    m.insert("General Collections", "Periodicals & Reference");
    m.insert("Art & Architecture", "Arts & Recreation");
    m.insert("Music", "Arts & Recreation");
    m.insert("Fine Arts", "Arts & Recreation");
    m.insert("Sports & Recreation", "Arts & Recreation");
    m.insert("Science & Nature", "Science & Technology");
    m.insert("Technology & Engineering", "Science & Technology");
    m.insert("Mathematics & Computing", "Science & Technology");
    m.insert("Economics & Business", "Social Sciences");
    m.insert("Sociology", "Social Sciences");
    m.insert("Politics & Government", "Law & Government");
    m.insert("Travel & Exploration", "History & Geography");
    m
});

static RE_LC_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z]{1,3}\d*$").unwrap());
static RE_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([A-Z]{1,3})").unwrap());
static RE_AGENT_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"agents/(\d+)").unwrap());
static RE_FILES_DIRS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:files|dirs)/([^/]+)/(.+)").unwrap());
static RE_SHELF_CAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^Category:\s*").unwrap());

// ---------------------------------------------------------------------------
// Structs & Models (Zero-Cost Serialization Pruning)
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug, Clone)]
pub struct Agent {
    #[serde(rename = "type")]
    pub agent_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<u64>,
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webpage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub death_date: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Format {
    #[serde(rename = "type")]
    pub mime_type: String,
    pub url: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Topic {
    pub heading: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub subtopics: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Taxonomy {
    pub domain: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub topics: Vec<Topic>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Ebook {
    pub title: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub alternative_titles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_date: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub agents: Vec<Agent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub language: String,
    pub formats: Vec<Format>,
    pub taxonomy: Taxonomy,
    pub downloads: u64,
    pub ebook_id: String,
    pub cover_image: String,
    pub license: String,
}

// ---------------------------------------------------------------------------
// Bridge Models: field names follow the target database schema (alt-target-schema.md)
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug, Clone)]
pub struct BridgeAgent {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pg_id: Option<u64>,
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub external_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub death_date: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct BridgeFormat {
    pub mime_type: String,
    pub file_url: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct BridgeEbook {
    pub title: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub alternative_titles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_date: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub agents: Vec<BridgeAgent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub lang_code: String,
    pub formats: Vec<BridgeFormat>,
    pub taxonomy: Taxonomy,
    pub pg_download_count: u64,
    pub pg_id: u64,
    pub md_cover_image_url: String,
    pub license_statement: String,
}

impl From<&Agent> for BridgeAgent {
    fn from(agent: &Agent) -> Self {
        BridgeAgent {
            role: agent.agent_type.clone(),
            pg_id: agent.agent_id,
            name: agent.name.clone(),
            aliases: agent.aliases.clone(),
            external_urls: agent.webpage.clone().into_iter().collect(),
            birth_date: agent.birth_date.clone(),
            death_date: agent.death_date.clone(),
        }
    }
}

impl From<&Format> for BridgeFormat {
    fn from(format: &Format) -> Self {
        BridgeFormat {
            mime_type: format.mime_type.clone(),
            file_url: format.url.clone(),
        }
    }
}

impl From<&Ebook> for BridgeEbook {
    fn from(ebook: &Ebook) -> Self {
        BridgeEbook {
            title: ebook.title.clone(),
            alternative_titles: ebook.alternative_titles.clone(),
            issued_date: ebook.issued_date.clone(),
            agents: ebook.agents.iter().map(BridgeAgent::from).collect(),
            description: ebook.description.clone(),
            lang_code: ebook.language.clone(),
            formats: ebook.formats.iter().map(BridgeFormat::from).collect(),
            taxonomy: ebook.taxonomy.clone(),
            pg_download_count: ebook.downloads,
            pg_id: ebook.ebook_id.parse::<u64>().unwrap_or(0),
            md_cover_image_url: ebook.cover_image.clone(),
            license_statement: ebook.license.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers & Data Logic
// ---------------------------------------------------------------------------

fn is_public_domain_license(license: &str) -> bool {
    license.to_lowercase().contains("public domain")
}

fn transform_url(url: Option<&str>, ebook_id: &str, mirror_base: &str) -> Option<String> {
    let url = url?;
    if url.is_empty() || mirror_base.is_empty() {
        return Some(url.to_string());
    }

    let mirror_clean = format!("{}/", mirror_base.trim_end_matches('/'));
    if mirror_clean == "https://www.gutenberg.org/" || mirror_clean == "http://www.gutenberg.org/" {
        return Some(url.to_string());
    }

    let ebook_id_clean = ebook_id.trim();
    let digit_path = if ebook_id_clean.len() <= 1 || !ebook_id_clean.chars().all(|c| c.is_ascii_digit()) {
        ebook_id_clean.to_string()
    } else {
        let chars: Vec<char> = ebook_id_clean.chars().collect();
        let prefix_parts: Vec<String> = chars[..chars.len() - 1].iter().map(|c| c.to_string()).collect();
        format!("{}/{}", prefix_parts.join("/"), ebook_id_clean)
    };

    if url.contains("/files/") || url.contains("/dirs/") {
        if let Some(caps) = RE_FILES_DIRS.captures(url) {
            if &caps[1] == ebook_id_clean {
                return Some(format!("{}{}/{}", mirror_clean, digit_path, &caps[2]));
            }
        }
    }

    let prefixes = [
        "https://www.gutenberg.org/",
        "http://www.gutenberg.org/",
        "https://gutenberg.org/",
        "http://gutenberg.org/",
    ];

    for prefix in prefixes {
        if let Some(rel_path) = url.strip_prefix(prefix) {
            if let Some(file_part) = rel_path.strip_prefix("ebooks/") {
                return Some(format!("{}cache/epub/{}/pg{}", mirror_clean, ebook_id_clean, file_part));
            }
            return Some(format!("{}{}", mirror_clean, rel_path));
        }
    }

    Some(url.to_string())
}

fn parse_lc_code(s: &str) -> Option<(&'static str, &'static str)> {
    let code = s.trim().to_uppercase();
    if !RE_LC_CODE.is_match(&code) {
        return None;
    }

    let prefix = RE_PREFIX.captures(&code)?.get(1)?.as_str();
    if let Some(&res) = LC_MAP.get(prefix) {
        return Some(res);
    }
    if prefix.len() > 1 {
        if let Some(&res) = LC_MAP.get(&prefix[..2]) {
            return Some(res);
        }
    }
    if !prefix.is_empty() {
        if let Some(&res) = LC_MAP.get(&prefix[..1]) {
            return Some(res);
        }
    }
    None
}

fn extract_taxonomy(subjects_raw: &[String], bookshelves_raw: &[String]) -> Taxonomy {
    let mut lc_domains = HashSet::new();
    let mut inferred_domains = HashSet::new();
    let mut genres = HashSet::new();
    let mut raw_topics = Vec::new();

    for subj in subjects_raw {
        let subj_clean = subj.trim();
        if subj_clean.is_empty() {
            continue;
        }

        if let Some((dom, gen)) = parse_lc_code(subj_clean) {
            lc_domains.insert(dom);
            genres.insert(gen);
        } else {
            let parts: Vec<&str> = subj_clean
                .split(" -- ")
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .collect();
            if !parts.is_empty() {
                let heading = parts[0];
                let heading_lower = heading.to_lowercase();
                if let Some(&(dom, gen)) = LCSH_FORM_GENRE_MAP.get(heading_lower.as_str()) {
                    inferred_domains.insert(dom);
                    genres.insert(gen);
                }
                let mut filtered_subtopics = Vec::new();
                for p in &parts[1..] {
                    let p_lower = p.to_lowercase();
                    if let Some(&(dom, gen)) = LCSH_FORM_GENRE_MAP.get(p_lower.as_str()) {
                        inferred_domains.insert(dom);
                        genres.insert(gen);
                    } else {
                        filtered_subtopics.push(p.to_string());
                    }
                }
                let mut topic_entry = vec![heading.to_string()];
                topic_entry.extend(filtered_subtopics);
                raw_topics.push(topic_entry);
            }
        }
    }

    for shelf in bookshelves_raw {
        let shelf_clean = RE_SHELF_CAT.replace(shelf, "").trim().to_string();
        let shelf_lower = shelf_clean.to_lowercase();

        if ["best books ever listings", "novels", "general"].contains(&shelf_lower.as_str()) {
            continue;
        }

        for (regex, genre_label) in BOOKSHELF_MAP.iter() {
            if regex.is_match(&shelf_lower) {
                genres.insert(*genre_label);
            }
        }
    }

    for genre in &genres {
        if let Some(&dom) = GENRE_TO_DOMAIN_MAP.get(genre) {
            inferred_domains.insert(dom);
        }
    }

    let primary_domain = if !lc_domains.is_empty() {
        let mut sorted: Vec<_> = lc_domains.into_iter().collect();
        sorted.sort();
        sorted[0]
    } else if !inferred_domains.is_empty() {
        let mut sorted: Vec<_> = inferred_domains.into_iter().collect();
        sorted.sort();
        sorted[0]
    } else {
        "General & Uncategorized"
    };

    let mut formatted_topics = Vec::new();
    let mut seen_keys = HashSet::new();

    for parts in raw_topics {
        let heading = parts[0].clone();
        let subtopics = parts[1..].to_vec();
        let key = (heading.clone(), subtopics.clone());

        if !seen_keys.contains(&key) {
            seen_keys.insert(key);
            formatted_topics.push(Topic { heading, subtopics });
        }
    }

    let mut sorted_genres: Vec<String> = genres.into_iter().map(|s| s.to_string()).collect();
    sorted_genres.sort();

    Taxonomy {
        domain: primary_domain.to_string(),
        genres: sorted_genres,
        topics: formatted_topics,
    }
}

// ---------------------------------------------------------------------------
// XML Parser Logic
// ---------------------------------------------------------------------------

fn parse_agent(parent_node: Option<Node>, agent_type: &str, ebook_id: &str, mirror_base: &str) -> Option<Agent> {
    let parent = parent_node?;
    let agent_node =
        if parent.tag_name().name() == "agent" && parent.tag_name().namespace() == NAMESPACES.get("pgterms").copied() {
            parent
        } else {
            parent
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "agent")?
        };

    let about = agent_node.attribute((NAMESPACES["rdf"], "about")).unwrap_or("");
    let agent_id = RE_AGENT_ID
        .captures(about)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok());

    let name = agent_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "name")
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim();

    if name.is_empty() {
        return None;
    }

    let aliases: Vec<String> = agent_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "alias")
        .filter_map(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let webpage = agent_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "webpage")
        .and_then(|n| n.attribute((NAMESPACES["rdf"], "resource")))
        .and_then(|url| transform_url(Some(url), ebook_id, mirror_base));

    let birth_date = agent_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "birthdate")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string());

    let death_date = agent_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "deathdate")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string());

    Some(Agent {
        agent_type: agent_type.to_string(),
        agent_id,
        name: name.to_string(),
        aliases,
        webpage,
        birth_date,
        death_date,
    })
}

fn process_rdf_xml(xml_data: &[u8], mirror_base: &str, include_licensed: bool) -> Result<Ebook, &'static str> {
    let xml_str = std::str::from_utf8(xml_data).map_err(|_| "utf8_error")?;
    let doc = Document::parse(xml_str).map_err(|_| "xml_parse_error")?;

    let root = doc.root_element();
    let ebook = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "ebook")
        .ok_or("no_ebook_element")?;

    let ebook_about = ebook.attribute((NAMESPACES["rdf"], "about")).unwrap_or("");
    let ebook_id = ebook_about.replace("ebooks/", "").trim().to_string();

    let type_val = ebook
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "type")
        .and_then(|n| n.descendants().find(|c| c.tag_name().name() == "value"))
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if type_val != "text" {
        return Err("filter_type");
    }

    let title = ebook
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "title")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or("filter_title")?;

    let language = ebook
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "language")
        .and_then(|n| n.descendants().find(|c| c.tag_name().name() == "value"))
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or("filter_language")?;

    let license = ebook
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "rights")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .unwrap_or_else(|| "Public domain in the USA.".to_string());

    if !include_licensed && !is_public_domain_license(&license) {
        return Err("filter_license");
    }

    let mut seen_mime = HashSet::new();
    let mut format_objects = Vec::new();

    for has_fmt in ebook
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "hasFormat")
    {
        if let Some(file_node) = has_fmt
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "file")
        {
            let file_url = file_node
                .attribute((NAMESPACES["rdf"], "about"))
                .or_else(|| file_node.attribute((NAMESPACES["rdf"], "resource")))
                .unwrap_or("");

            let fmt_val = file_node
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "value")
                .and_then(|n| n.text())
                .unwrap_or("")
                .trim();

            if let Some(transformed_url) = transform_url(Some(file_url), &ebook_id, mirror_base) {
                let url_lower = transformed_url.to_lowercase();
                let mime_key =
                    if fmt_val.contains("text/html") || url_lower.ends_with(".htm") || url_lower.ends_with(".html") {
                        Some("text/html")
                    } else if fmt_val.contains("epub") || url_lower.ends_with(".epub") {
                        Some("application/epub+zip")
                    } else {
                        None
                    };

                if let Some(key) = mime_key {
                    if seen_mime.insert(key) {
                        format_objects.push(Format {
                            mime_type: key.to_string(),
                            url: transformed_url,
                        });
                    }
                }
            }
        }
    }

    if !seen_mime.contains("text/html") || !seen_mime.contains("application/epub+zip") {
        return Err("filter_required_formats");
    }

    let mut agents = Vec::new();
    let push_agents = |tag: &str, agent_type: &str, agents: &mut Vec<Agent>| {
        for node in ebook
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == tag)
        {
            if let Some(agent) = parse_agent(Some(node), agent_type, &ebook_id, mirror_base) {
                agents.push(agent);
            }
        }
    };

    push_agents("creator", "author", &mut agents);
    push_agents("trl", "translator", &mut agents);
    push_agents("aui", "introduction_author", &mut agents);
    push_agents("ill", "illustrator", &mut agents);
    if !agents.iter().any(|a| a.agent_type == "author") {
        push_agents("aut", "author", &mut agents);
    }

    if agents.is_empty() {
        return Err("filter_creator");
    }

    let cover_image = transform_url(
        Some(&format!(
            "https://www.gutenberg.org/cache/epub/{}/pg{}.cover.medium.jpg",
            ebook_id, ebook_id
        )),
        &ebook_id,
        mirror_base,
    )
    .unwrap();

    let description = ebook
        .children()
        .find(|n| ["marc520", "marc500", "description"].contains(&n.tag_name().name()))
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string());

    let alternative_titles: Vec<String> = ebook
        .children()
        .filter(|n| n.tag_name().name() == "alternative")
        .filter_map(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let issued_date = ebook
        .children()
        .find(|n| n.tag_name().name() == "issued")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string());

    let downloads = ebook
        .children()
        .find(|n| n.tag_name().name() == "downloads")
        .and_then(|n| n.text())
        .and_then(|t| t.trim().parse::<u64>().ok())
        .unwrap_or(0);

    let subjects_raw: Vec<String> = ebook
        .children()
        .filter(|n| n.tag_name().name() == "subject")
        .filter_map(|n| n.descendants().find(|c| c.tag_name().name() == "value"))
        .filter_map(|n| n.text())
        .map(|t| t.trim().to_string())
        .collect();

    let bookshelves_raw: Vec<String> = ebook
        .children()
        .filter(|n| n.tag_name().name() == "bookshelf")
        .filter_map(|n| n.descendants().find(|c| c.tag_name().name() == "value"))
        .filter_map(|n| n.text())
        .map(|t| t.trim().to_string())
        .collect();

    let taxonomy = extract_taxonomy(&subjects_raw, &bookshelves_raw);

    Ok(Ebook {
        title,
        alternative_titles,
        issued_date,
        agents,
        description,
        language,
        formats: format_objects,
        taxonomy,
        downloads,
        ebook_id,
        cover_image,
        license,
    })
}

// ---------------------------------------------------------------------------
// MPMC Multi-Threaded CLI Entry Point
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(author, version, about = "Ultra-fast Multi-threaded Gutenberg Archive Extractor")]
struct Args {
    archive_path: String,
    #[arg(short, long, default_value = "filtered_ebooks.json")]
    output: String,
    #[arg(short, long, default_value = "gutenberg")]
    mirror: String,
    #[arg(long)]
    max_results: Option<usize>,
    #[arg(short, long)]
    chunk_size: Option<usize>,
    #[arg(
        long,
        help = "Rename output object fields to match the target database schema (alt-target-schema.md)"
    )]
    bridge: bool,
    #[arg(
        long,
        help = "Also include ebooks that are NOT Public Domain (copyrighted or otherwise licensed)"
    )]
    include_licensed: bool,
}

fn write_chunk(data: &mut [Ebook], path: &str, bridge: bool) -> std::io::Result<()> {
    // Sort deterministically by numeric ebook_id
    data.sort_by_key(|e| e.ebook_id.parse::<u64>().unwrap_or(u64::MAX));

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let write_array = |w: &mut dyn Write| -> std::io::Result<()> {
        w.write_all(b"[\n")?;
        let total = data.len();
        for (i, ebook) in data.iter().enumerate() {
            if bridge {
                serde_json::to_writer(&mut *w, &BridgeEbook::from(ebook))?;
            } else {
                serde_json::to_writer(&mut *w, ebook)?;
            }
            if i + 1 < total {
                w.write_all(b",\n")?;
            } else {
                w.write_all(b"\n")?;
            }
        }
        w.write_all(b"]\n")?;
        Ok(())
    };

    if path.ends_with(".gz") {
        let mut encoder = GzEncoder::new(writer, Compression::default());
        write_array(&mut encoder)?;
        encoder.finish()?;
    } else {
        write_array(&mut writer)?;
    }
    Ok(())
}

fn get_chunk_path(base_path: &str, chunk_index: usize) -> String {
    if let Some(stripped) = base_path.strip_suffix(".json.gz") {
        format!("{}_{}.json.gz", stripped, chunk_index)
    } else if let Some(pos) = base_path.rfind('.') {
        format!("{}_{}.{}", &base_path[..pos], chunk_index, &base_path[pos + 1..])
    } else {
        format!("{}_{}", base_path, chunk_index)
    }
}

fn main() {
    let args = Args::parse();
    let start_time = Instant::now();

    let mirror_base = GUTENBERG_MIRRORS
        .get(args.mirror.to_lowercase().as_str())
        .unwrap_or(&args.mirror.as_str())
        .to_string();

    println!("[INFO] Opening archive: {}", args.archive_path);
    println!("[INFO] Using mirror base: {}", mirror_base);
    if args.bridge {
        println!("[INFO] Bridge output mode enabled: field names match the target database schema");
    }
    if args.include_licensed {
        println!("[INFO] Including non-Public-Domain (licensed) ebooks");
    } else {
        println!("[INFO] Filtering to Public Domain ebooks only");
    }

    let (raw_tx, raw_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = bounded(2048);
    let (parsed_tx, parsed_rx): (Sender<Ebook>, Receiver<Ebook>) = bounded(2048);

    // Producer Thread: Single-pass bz2 stream decompression
    let archive_path = args.archive_path.clone();
    std::thread::spawn(move || {
        let file = File::open(&archive_path).expect("Failed to open archive file");
        let buf_reader = BufReader::with_capacity(1024 * 1024, file);
        let bz_decoder = BzDecoder::new(buf_reader);
        let mut archive = Archive::new(bz_decoder);

        if let Ok(entries) = archive.entries() {
            for entry in entries.flatten() {
                let name = entry.path().unwrap_or_default().to_string_lossy().to_string();
                if name.ends_with(".xml") || name.ends_with(".rdf") {
                    let mut buffer = Vec::new();
                    let mut entry = entry;
                    if entry.read_to_end(&mut buffer).is_ok() && raw_tx.send(buffer).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Worker Threads (std::thread Pool)
    let num_workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    println!("[INFO] Spawning {} parallel worker threads", num_workers);

    for _ in 0..num_workers {
        let rx = raw_rx.clone();
        let tx = parsed_tx.clone();
        let mirror = mirror_base.clone();
        let include_licensed = args.include_licensed;

        std::thread::spawn(move || {
            while let Ok(raw_bytes) = rx.recv() {
                if let Ok(ebook) = process_rdf_xml(&raw_bytes, &mirror, include_licensed) {
                    if tx.send(ebook).is_err() {
                        break;
                    }
                }
            }
        });
    }
    drop(parsed_tx); // Close remaining producer reference

    // Consumer Thread: Streaming Output Collector & Writer
    let mut current_chunk = Vec::new();
    let mut chunk_index = 1;
    let mut total_matched = 0;

    while let Ok(ebook) = parsed_rx.recv() {
        current_chunk.push(ebook);
        total_matched += 1;

        if let Some(c_size) = args.chunk_size {
            if current_chunk.len() >= c_size {
                let path = get_chunk_path(&args.output, chunk_index);
                write_chunk(&mut current_chunk, &path, args.bridge).expect("Failed to write chunk");
                println!(
                    "[INFO] Flushed chunk {} ({} items) -> {}",
                    chunk_index,
                    current_chunk.len(),
                    path
                );
                chunk_index += 1;
                current_chunk.clear();
            }
        }

        if let Some(max) = args.max_results {
            if total_matched >= max {
                break;
            }
        }
    }

    if !current_chunk.is_empty() {
        if args.chunk_size.is_some() {
            let path = get_chunk_path(&args.output, chunk_index);
            write_chunk(&mut current_chunk, &path, args.bridge).expect("Failed to write final chunk");
            println!(
                "[INFO] Flushed final chunk {} ({} items) -> {}",
                chunk_index,
                current_chunk.len(),
                path
            );
        } else {
            write_chunk(&mut current_chunk, &args.output, args.bridge).expect("Failed to write output");
            println!("[INFO] Wrote {} matched items -> {}", total_matched, args.output);
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!(
        "[INFO] Pipeline complete in {:.2}s. Total Matched: {}",
        elapsed, total_matched
    );
}
