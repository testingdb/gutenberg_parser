//! Static Configuration Tables & Regex Patterns
//! ---------------------------------------------------------
//! This module contains all immutable lookup data and compiled regex
//! patterns used across the parser pipeline. Everything is loaded lazily
//! via `LazyLock` (or as `const` where appropriate) so initialization
//! happens only on first access, avoiding startup overhead for unused
//! tables.
//!
//! ## Tables
//! - `NAMESPACES`: RDF namespace URIs keyed by prefix.
//! - `GUTENBERG_MIRRORS`: Mirror base URLs (keyed by user-facing name).
//! - `LC_MAP`: Library of Congress classification codes mapped to
//!   `(Domain, Sub-domain / Description)` tuples.
//! - `BOOKSHELF_MAP`: Ordered regex-to-label mappings for Gutenberg
//!   bookshelf categories.
//! - `LCSH_FORM_GENRE_MAP`: Form / genre keywords mapped to
//!   `(Broad Domain, Narrow Genre)` tuples.
//! - `GENRE_TO_DOMAIN_MAP`: Inverse mapping from genre labels to broad
//!   taxonomy domains.
//!
//! ## Regex Patterns
//! Compiled once via `LazyLock<Regex>`; reused in parsing and URL
//! transformation logic.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Namespace Definitions (RDF / Dublin Core / PG Terms / MARC Relator)
// ---------------------------------------------------------------------------

/// RDF namespace prefixes mapped to their canonical URIs.
///
/// Used by `xml_parser.rs` when resolving qualified XML tag names and
/// resource attributes.
pub static NAMESPACES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
    m.insert("dcterms", "http://purl.org/dc/terms/");
    m.insert("pgterms", "http://www.gutenberg.org/2009/pgterms/");
    m.insert("marcrel", "http://id.loc.gov/vocabulary/relators/");
    m.insert("cc", "http://web.resource.org/cc/");
    m.insert("rdfs", "http://www.w3.org/2000/01/rdf-schema#");
    m
});

/// Mirror URLs available for download and link transformation.
///
/// Keys are user-provided mirror names (`gutenberg`, `pglaf`, `odu`,
/// `waterloo`, `uk`, `xmission`). Values are the base URL strings.
pub static GUTENBERG_MIRRORS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
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

/// URL of the RDF archive feed provided by Project Gutenberg.
pub const RDF_FEED_URL: &str = "https://www.gutenberg.org/cache/epub/feeds/rdf-files.tar.bz2";

// ---------------------------------------------------------------------------
// Library of Congress Classification (LC_MAP)
// ---------------------------------------------------------------------------

/// Library of Congress codes mapped to `(Primary Domain, Sub-description)`.
///
/// Covers the full range from `A` (General Works) through `Z` (Bibliography),
/// including numeric sub-codes such as `E011`, `D501`, `D731`, and `F350.5`.
pub static LC_MAP: LazyLock<HashMap<&'static str, (&'static str, &'static str)>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // General Works
    m.insert("AC", ("General Works", "Collections, Series, Pamphlets"));
    m.insert("AE", ("General Works", "Encyclopedias"));
    m.insert("AG", ("General Works", "Dictionaries & Reference"));
    m.insert("AM", ("General Works", "Museums, Collectors and collecting"));
    m.insert("AP", ("General Works", "Periodicals"));
    m.insert("AS", ("General Works", "Academies, Associations, Congresses"));
    m.insert("AY", ("General Works", "Yearbooks, Almanacs, Directories"));
    m.insert("AZ", ("General Works", "History of Scholarship & Humanities"));
    // Philosophy & Religion
    m.insert("B", ("Philosophy & Religion", ""));
    m.insert("BC", ("Philosophy & Religion", "Logic"));
    m.insert("BD", ("Philosophy & Religion", "Speculative Philosophy"));
    m.insert("BF", ("Philosophy & Religion", "Psychology & Psychoanalysis"));
    m.insert("BH", ("Philosophy & Religion", "Aesthetics"));
    m.insert("BJ", ("Philosophy & Religion", "Ethics, Etiquette, Religion"));
    m.insert("BL", ("Philosophy & Religion", "Religion: General & Atheism"));
    m.insert("BM", ("Philosophy & Religion", "Judaism"));
    m.insert(
        "BP",
        ("Philosophy & Religion", "Islam, Bahaism, Theosophy, New Beliefs"),
    );
    m.insert("BQ", ("Philosophy & Religion", "Buddhism"));
    m.insert("BR", ("Philosophy & Religion", "Christianity"));
    m.insert("BS", ("Philosophy & Religion", "Christianity: Bible"));
    m.insert("BT", ("Philosophy & Religion", "Christianity: Doctrinal Theology"));
    m.insert("BV", ("Philosophy & Religion", "Christianity: Worship & Practice"));
    m.insert("BX", ("Philosophy & Religion", "Christianity: Churches & Movements"));
    // History
    m.insert("CB", ("History", "History of civilization"));
    m.insert("CC", ("History", "Archaeology"));
    m.insert("CE", ("History", "Technical Chronology, Calendar"));
    m.insert("CJ", ("History", "Numismatics"));
    m.insert("CN", ("History", "Inscriptions, Epigraphy"));
    m.insert("CR", ("History", "Heraldry"));
    m.insert("CS", ("History", "Genealogy"));
    m.insert("CT", ("History", "Biography"));
    m.insert("D", ("History", "General and Eastern Hemisphere"));
    m.insert("D501", ("History", "World War I"));
    m.insert("D731", ("History", "World War II"));
    m.insert("DA", ("History", "Great Britain, Ireland, Central Europe"));
    m.insert("DB", ("History", "Austria, Hungary, Czech Republic, Slovakia"));
    m.insert("DC", ("History", "France, Andorra, Monaco"));
    m.insert("DD", ("History", "Germany"));
    m.insert("DE", ("History", "Mediterranean & Greco-Roman World"));
    m.insert("DF", ("History", "Greece"));
    m.insert("DG", ("History", "Italy, Vatican City, Malta"));
    m.insert("DH", ("History", "Netherlands, Belgium, Luxembourg"));
    m.insert("DJ", ("History", "Netherlands"));
    m.insert("DJK", ("History", "Eastern Europe"));
    m.insert("DK", ("History", "Russia, Soviet Republics, Poland"));
    m.insert("DL", ("History", "Northern Europe, Scandinavia"));
    m.insert("DP", ("History", "Spain, Portugal"));
    m.insert("DQ", ("History", "Switzerland"));
    m.insert("DR", ("History", "Balkan Peninsula, Turkey"));
    m.insert("DS", ("History", "Asia"));
    m.insert("DT", ("History", "Africa"));
    m.insert("DU", ("History", "Oceania (South Seas)"));
    m.insert("DX", ("History", "Romanies"));
    // American History
    m.insert("E011", ("History", "Americas"));
    m.insert("E151", ("History", "United States"));
    m.insert("E186", ("History", "Colonial History"));
    m.insert("E201", ("History", "Revolution"));
    m.insert("E300", ("History", "Revolution to Civil War"));
    m.insert("E456", ("History", "Civil War"));
    m.insert("E660", ("History", "Late 19th Century"));
    m.insert("E740", ("History", "20th Century"));
    m.insert("E838", ("History", "Later 20th Century"));
    m.insert("E895", ("History", "21st Century"));
    // Regional / Local History
    m.insert("F001", ("History", "New England"));
    m.insert("F1001", ("History", "Canada"));
    m.insert("F106", ("History", "Atlantic & Middle Atlantic States"));
    m.insert("F1201", ("History", "Mexico"));
    m.insert("F1401", ("History", "Latin America"));
    m.insert("F1461", ("History", "Guatemala"));
    m.insert("F1481", ("History", "El Salvador"));
    m.insert("F1501", ("History", "Honduras"));
    m.insert("F1521", ("History", "Nicaragua"));
    m.insert("F1541", ("History", "Costa Rica"));
    m.insert("F1561", ("History", "Panama"));
    m.insert("F1601", ("History", "West Indies"));
    m.insert("F1751", ("History", "Cuba"));
    m.insert("F1861", ("History", "Jamaica"));
    m.insert("F1900", ("History", "Hispaniola"));
    m.insert("F1951", ("History", "Puerto Rico"));
    m.insert("F2001", ("History", "Lesser Antilles"));
    m.insert("F206", ("History", "South Atlantic States"));
    m.insert("F2131", ("History", "British West Indies"));
    m.insert("F2155", ("History", "Caribbean Area & Sea"));
    m.insert("F2201", ("History", "South America"));
    m.insert("F2251", ("History", "Colombia"));
    m.insert("F2301", ("History", "Venezuela"));
    m.insert("F2351", ("History", "Guyana"));
    m.insert("F2501", ("History", "Brazil"));
    m.insert("F2661", ("History", "Paraguay"));
    m.insert("F2701", ("History", "Uruguay"));
    m.insert("F2801", ("History", "Argentina"));
    m.insert("F296", ("History", "Gulf States & West Florida"));
    m.insert("F3051", ("History", "Chile"));
    m.insert("F3301", ("History", "Bolivia"));
    m.insert("F3401", ("History", "Peru"));
    m.insert("F350.5", ("History", "Mississippi Valley, Middle West"));
    m.insert("F3701", ("History", "Ecuador"));
    m.insert("F396", ("History", "Old Southwest, Lower Mississippi"));
    m.insert("F476", ("History", "Old Northwest, Northwest Territory"));
    m.insert("F516", ("History", "Ohio River Valley"));
    m.insert("F590.3", ("History", "Trans-Mississippi, Great Plains"));
    m.insert("F721", ("History", "Rocky Mountains, Yellowstone"));
    m.insert("F786", ("History", "New Southwest, Colorado River Valley"));
    m.insert("F850.5", ("History", "Pacific States"));
    m.insert("F975", ("History", "US-Protected Territories"));
    // Geography & Anthropology
    m.insert("G", ("Geography & Anthropology", ""));
    m.insert(
        "GA",
        ("Geography & Anthropology", "Mathematical Geography & Cartography"),
    );
    m.insert("GB", ("Geography & Anthropology", "Physical geography"));
    m.insert("GC", ("Geography & Anthropology", "Oceanography"));
    m.insert("GF", ("Geography & Anthropology", "Human Ecology"));
    m.insert("GN", ("Geography & Anthropology", "Anthropology"));
    m.insert("GR", ("Geography & Anthropology", "Folklore"));
    m.insert("GT", ("Geography & Anthropology", "Manners & Customs"));
    m.insert("GV", ("Geography & Anthropology", "Recreation & Leisure"));
    // Social sciences
    m.insert("H", ("Social sciences", ""));
    m.insert("HA", ("Social sciences", "Statistics"));
    m.insert("HB", ("Social sciences", "Economic Theory & Demography"));
    m.insert("HC", ("Social sciences", "Economic History & Conditions"));
    m.insert("HD", ("Social sciences", "Production"));
    m.insert("HE", ("Social sciences", "Transportation & Communications"));
    m.insert("HF", ("Social sciences", "Commerce"));
    m.insert("HG", ("Social sciences", "Finance"));
    m.insert("HJ", ("Social sciences", "Public finance"));
    m.insert("HM", ("Social sciences", "Sociology"));
    m.insert("HN", ("Social sciences", "Social History & Problems"));
    m.insert("HQ", ("Social sciences", "Family, Marriage, Gender"));
    m.insert("HS", ("Social sciences", "Secret & Benevolent Societies"));
    m.insert("HT", ("Social sciences", "Communities, Classes, Races"));
    m.insert("HV", ("Social sciences", "Social Pathology & Welfare"));
    m.insert("HX", ("Social sciences", "Socialism, Communism, Anarchism"));
    // Political Science
    m.insert("J", ("Political science", ""));
    m.insert("JA", ("Political science", "Political science"));
    m.insert("JC", ("Political science", "Political theory"));
    m.insert("JF", ("Political science", "Political Institutions & Administration"));
    m.insert("JK", ("Political science", "Political Admin.: US"));
    m.insert("JL", ("Political science", "Political Admin.: Americas"));
    m.insert("JN", ("Political science", "Political Admin.: Europe"));
    m.insert("JQ", ("Political science", "Political Admin.: Asia, Africa, Oceania"));
    m.insert("JS", ("Political science", "Local & Municipal Government"));
    m.insert("JV", ("Political science", "Colonization & Migration"));
    m.insert("JX", ("Political science", "International law"));
    m.insert("JZ", ("Political science", "International relations"));
    // Law
    m.insert("K", ("Law & Jurisprudence", ""));
    m.insert("KBM", ("Law & Jurisprudence", "Jewish law"));
    m.insert("KBR", ("Law & Jurisprudence", "Canon Law History"));
    m.insert("KD", ("Law & Jurisprudence", "UK & Ireland"));
    m.insert("KDZ", ("Law & Jurisprudence", "North America"));
    m.insert("KE", ("Law & Jurisprudence", "Canada"));
    m.insert("KF", ("Law & Jurisprudence", "United States"));
    m.insert("KH", ("Law & Jurisprudence", "South America"));
    m.insert("KJ", ("Law & Jurisprudence", "Europe"));
    m.insert("KL", ("Law & Jurisprudence", "Asia, Africa, Pacific & Antarctica"));
    m.insert("KN", ("Law & Jurisprudence", "South & East Asia"));
    m.insert("KNX", ("Law & Jurisprudence", "Japan"));
    m.insert("KP", ("Law & Jurisprudence", "South & East Asia"));
    m.insert("KZ", ("Law & Jurisprudence", "Law of nations"));
    // Education
    m.insert("L", ("Education", ""));
    m.insert("LA", ("Education", "History of education"));
    m.insert("LB", ("Education", "Education Theory & Practice"));
    m.insert("LC", ("Education", "Special Education Aspects"));
    m.insert("LD", ("Education", "US Institutions"));
    m.insert("LE", ("Education", "Americas (excl. US)"));
    m.insert("LF", ("Education", "European Institutions"));
    m.insert("LH", ("Education", "School Magazines & Papers"));
    m.insert("LT", ("Education", "Textbooks"));
    // Music
    m.insert("M", ("Music", ""));
    m.insert("ML", ("Music", "Literature of music"));
    m.insert("MT", ("Music", "Music Instruction & Composition"));
    // Fine Arts
    m.insert("N", ("Fine Arts", ""));
    m.insert("NA", ("Fine Arts", "Architecture"));
    m.insert("NB", ("Fine Arts", "Sculpture"));
    m.insert("NC", ("Fine Arts", "Drawing, Design, Illustration"));
    m.insert("ND", ("Fine Arts", "Painting"));
    m.insert("NE", ("Fine Arts", "Print media"));
    m.insert("NK", ("Fine Arts", "Decorative & Applied Arts"));
    m.insert("NX", ("Fine Arts", "Arts in general"));
    // Language & Literature
    m.insert("P", ("Language & Literature", ""));
    m.insert("PA", ("Language & Literature", "Classical Languages & Literature"));
    m.insert("PB", ("Language & Literature", "General works"));
    m.insert("PC", ("Language & Literature", "Romance Languages"));
    m.insert("PD", ("Language & Literature", "Germanic & Scandinavian"));
    m.insert("PE", ("Language & Literature", "English"));
    m.insert("PF", ("Language & Literature", "West Germanic"));
    m.insert("PG", ("Language & Literature", "Slavic & Russian"));
    m.insert("PH", ("Language & Literature", "Finno-Ugrian & Basque"));
    m.insert("PJ", ("Language & Literature", "Oriental Languages"));
    m.insert("PK", ("Language & Literature", "Indo-Iranian"));
    m.insert("PL", ("Language & Literature", "East Asia, Africa, Oceania"));
    m.insert("PM", ("Language & Literature", "Indigenous & Artificial Languages"));
    m.insert("PN", ("Language & Literature", "Literature: General, Criticism"));
    m.insert("PQ", ("Language & Literature", "Romance Literatures"));
    m.insert("PR", ("Language & Literature", "English literature"));
    m.insert("PS", ("Language & Literature", "American & Canadian"));
    m.insert("PT", ("Language & Literature", "Germanic, Scandinavian, Icelandic"));
    m.insert("PZ", ("Language & Literature", "Juvenile Literature"));
    // Science
    m.insert("Q", ("Science", ""));
    m.insert("QA", ("Science", "Mathematics"));
    m.insert("QB", ("Science", "Astronomy"));
    m.insert("QC", ("Science", "Physics"));
    m.insert("QD", ("Science", "Chemistry"));
    m.insert("QE", ("Science", "Geology"));
    m.insert("QH", ("Science", "Natural history"));
    m.insert("QH301", ("Science", "Biology"));
    m.insert("QK", ("Science", "Botany"));
    m.insert("QL", ("Science", "Zoology"));
    m.insert("QM", ("Science", "Human anatomy"));
    m.insert("QP", ("Science", "Physiology"));
    m.insert("QR", ("Science", "Microbiology"));
    // Medicine
    m.insert("R", ("Medicine", ""));
    m.insert("RA", ("Medicine", "Public aspects of medicine"));
    m.insert("RB", ("Medicine", "Pathology"));
    m.insert("RC", ("Medicine", "Internal medicine"));
    m.insert("RD", ("Medicine", "Surgery"));
    m.insert("RE", ("Medicine", "Ophthalmology"));
    m.insert("RF", ("Medicine", "Otorhinolaryngology"));
    m.insert("RG", ("Medicine", "Gynecology and obstetrics"));
    m.insert("RJ", ("Medicine", "Pediatrics"));
    m.insert("RK", ("Medicine", "Dentistry"));
    m.insert("RL", ("Medicine", "Dermatology"));
    m.insert("RM", ("Medicine", "Therapeutics, Pharmacology"));
    m.insert("RS", ("Medicine", "Pharmacy and materia medica"));
    m.insert("RT", ("Medicine", "Nursing"));
    m.insert("RV", ("Medicine", "Botanic & Eclectic Medicine"));
    m.insert("RX", ("Medicine", "Homeopathy"));
    m.insert("RZ", ("Medicine", "Other systems of medicine"));
    // Agriculture
    m.insert("S", ("Agriculture", ""));
    m.insert("SB", ("Agriculture", "Plant culture"));
    m.insert("SD", ("Agriculture", "Forestry"));
    m.insert("SF", ("Agriculture", "Animal culture"));
    m.insert("SH", ("Agriculture", "Aquaculture, Fisheries, Angling"));
    m.insert("SK", ("Agriculture", "Hunting sports"));
    // Technology
    m.insert("T", ("Technology", ""));
    m.insert("TA", ("Technology", "Engineering & Civil"));
    m.insert("TC", ("Technology", "Ocean engineering"));
    m.insert("TD", ("Technology", "Environmental & Sanitary"));
    m.insert("TE", ("Technology", "Highway & Roads"));
    m.insert("TF", ("Technology", "Railroads"));
    m.insert("TG", ("Technology", "Bridge engineering"));
    m.insert("TH", ("Technology", "Building construction"));
    m.insert("TJ", ("Technology", "Mechanical Engineering"));
    m.insert("TK", ("Technology", "Electrical, Electronics & Nuclear"));
    m.insert("TL", ("Technology", "Vehicles, Aeronautics & Space"));
    m.insert("TN", ("Technology", "Mining & Metallurgy"));
    m.insert("TP", ("Technology", "Chemical technology"));
    m.insert("TR", ("Technology", "Photography"));
    m.insert("TS", ("Technology", "Manufactures"));
    m.insert("TT", ("Technology", "Handicrafts & Crafts"));
    m.insert("TX", ("Technology", "Home economics"));
    // Military Science
    m.insert("U", ("Military science", ""));
    m.insert("UA", ("Military science", "Armies & Organization"));
    m.insert("UB", ("Military science", "Military administration"));
    m.insert("UC", ("Military science", "Maintenance & Transport"));
    m.insert("UD", ("Military science", "Infantry"));
    m.insert("UE", ("Military science", "Cavalry, Armor"));
    m.insert("UF", ("Military science", "Artillery"));
    m.insert("UG", ("Military science", "Military engineering"));
    m.insert("UH", ("Military science", "Other services"));
    // Naval Science
    m.insert("V", ("Naval science", ""));
    m.insert("VA", ("Naval science", "Navies & Organization"));
    m.insert("VB", ("Naval science", "Naval administration"));
    m.insert("VE", ("Naval science", "Marines"));
    m.insert("VF", ("Naval science", "Naval ordnance"));
    m.insert("VG", ("Naval science", "Minor Naval Services"));
    m.insert("VK", ("Naval science", "Navigation & Merchant Marine"));
    m.insert("VM", ("Naval science", "Naval Architecture & Engineering"));
    // Bibliography / Library Science
    m.insert("Z", ("Bibliography & Library Science", ""));
    m
});

// ---------------------------------------------------------------------------
// Bookshelf Genre Mapping
// ---------------------------------------------------------------------------

/// Bookshelf label regexes mapped to standardized genre labels.
///
/// Ordered by priority; the first regex that matches a shelf label wins.
/// Used by `taxonomy.rs` to infer `genres` from Gutenberg shelf data.
pub static BOOKSHELF_MAP: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
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

/// LCSH (Library of Congress Subject Headings) form/genre keywords mapped
/// to `(Broad Domain, Narrow Genre)` tuples.
///
/// Used by `taxonomy.rs` to infer taxonomy from subject lines in RDF.
pub static LCSH_FORM_GENRE_MAP: LazyLock<HashMap<&'static str, (&'static str, &'static str)>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("fiction", ("Language & Literature", "Fiction & Novels"));
    m.insert("juvenile fiction", ("Language & Literature", "Children's Literature"));
    m.insert(
        "juvenile literature",
        ("Language & Literature", "Children's Literature"),
    );
    m.insert("juvenile poetry", ("Language & Literature", "Children's Literature"));
    m.insert("biography", ("History", "Biography & Memoir"));
    m.insert("autobiography", ("History", "Biography & Memoir"));
    m.insert("humor", ("Social sciences", "Humor & Satire"));
    m.insert("satire", ("Social sciences", "Humor & Satire"));
    m.insert("drama", ("Language & Literature", "Drama & Theater"));
    m.insert("poetry", ("Language & Literature", "Poetry"));
    m.insert("short stories", ("Language & Literature", "Fiction & Novels"));
    m.insert("memoirs", ("History", "Biography & Memoir"));
    m.insert("memoir", ("History", "Biography & Memoir"));
    m.insert("diaries", ("History", "Biography & Memoir"));
    m.insert("letters", ("Language & Literature", "Essays & Literary Collections"));
    m.insert("speeches", ("Language & Literature", "Essays & Literary Collections"));
    m.insert("interviews", ("General Works", "Journalism & Media"));
    m.insert(
        "correspondence",
        ("Language & Literature", "Essays & Literary Collections"),
    );
    m.insert("essays", ("Language & Literature", "Essays & Literary Collections"));
    m.insert(
        "science fiction",
        ("Language & Literature", "Science Fiction & Fantasy"),
    );
    m.insert(
        "fantasy fiction",
        ("Language & Literature", "Science Fiction & Fantasy"),
    );
    m.insert("fantasy", ("Language & Literature", "Science Fiction & Fantasy"));
    m.insert("horror", ("Language & Literature", "Horror & Gothic"));
    m.insert("mystery", ("Language & Literature", "Mystery & Crime"));
    m.insert(
        "detective and mystery fiction",
        ("Language & Literature", "Mystery & Crime"),
    );
    m.insert("romance fiction", ("Language & Literature", "Fiction & Novels"));
    m.insert("historical fiction", ("Language & Literature", "Fiction & Novels"));
    m.insert("war stories", ("History", "History"));
    m.insert("adventure stories", ("Language & Literature", "Action & Adventure"));
    m.insert("journals", ("General Works", "Periodicals & Journals"));
    m.insert("notebooks", ("Language & Literature", "Essays & Literary Collections"));
    // Extension for books escaping classification filters
    m.insert("catalogs", ("Bibliography & Library Science", "Catalogs"));
    m.insert("catalog", ("Bibliography & Library Science", "Catalogs"));
    m.insert("periodicals", ("General Works", "Periodicals & Journals"));
    m.insert("dictionaries", ("General Works", "Dictionaries & Reference"));
    m.insert("dictionary", ("General Works", "Dictionaries & Reference"));
    m.insert("handbooks", ("General Works", "Handbooks & Manuals"));
    m.insert("manuals", ("General Works", "Handbooks & Manuals"));
    m.insert("handbooks, manuals, etc", ("General Works", "Handbooks & Manuals"));
    m.insert("handbooks, manuals, etc.", ("General Works", "Handbooks & Manuals"));
    m.insert("early works to 1800", ("Language & Literature", "Early Works"));
    m.insert("reference", ("General Works", "Reference Works"));
    m.insert("bibliography", ("Bibliography & Library Science", "Bibliography"));
    m.insert("bibliographies", ("Bibliography & Library Science", "Bibliography"));
    m
});

/// Inverse mapping from genre label to broad taxonomy domain.
///
/// Enables `taxonomy.rs` to resolve `inferred_domains` from genres.
pub static GENRE_TO_DOMAIN_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("Science Fiction & Fantasy", "Language & Literature");
    m.insert("Fiction & Novels", "Language & Literature");
    m.insert("British Literature", "Language & Literature");
    m.insert("American Literature", "Language & Literature");
    m.insert("Classics", "Language & Literature");
    m.insert("Drama & Theater", "Language & Literature");
    m.insert("Action & Adventure", "Language & Literature");
    m.insert("Horror & Gothic", "Language & Literature");
    m.insert("Mystery & Crime", "Language & Literature");
    m.insert("Poetry", "Language & Literature");
    m.insert("Children's Literature", "Language & Literature");
    m.insert("Humor & Satire", "Social sciences");
    m.insert("Essays & Literary Collections", "Language & Literature");
    m.insert("Classical Literature", "Language & Literature");
    m.insert("French, Italian & Spanish Literature", "Language & Literature");
    m.insert("German & Nordic Literature", "Language & Literature");
    m.insert("British History", "History");
    m.insert("Medieval History", "History");
    m.insert("American History", "History");
    m.insert("European History", "History");
    m.insert("Modern History", "History");
    m.insert("French History", "History");
    m.insert("German History", "History");
    m.insert("General World History", "History");
    m.insert("Biography & Memoir", "History");
    m.insert("Law & Legal Studies", "Law & Jurisprudence");
    m.insert("British Law", "Law & Jurisprudence");
    m.insert("United States Law", "Law & Jurisprudence");
    m.insert("European Government", "Law & Jurisprudence");
    m.insert("Political Science", "Law & Jurisprudence");
    m.insert("Family & Relationships", "Social sciences");
    m.insert("Religion & Spirituality", "Philosophy & Religion");
    m.insert("Christianity", "Philosophy & Religion");
    m.insert("Philosophy", "Philosophy & Religion");
    m.insert("Journalism & Media", "General Works");
    m.insert("Periodicals & Journals", "General Works");
    m.insert("General Collections", "General Works");
    m.insert("Art & Architecture", "Fine Arts");
    m.insert("Music", "Fine Arts");
    m.insert("Fine Arts", "Fine Arts");
    m.insert("Sports & Recreation", "Fine Arts");
    m.insert("Science & Nature", "Science");
    m.insert("Technology & Engineering", "Science");
    m.insert("Mathematics & Computing", "Science");
    m.insert("Economics & Business", "Social sciences");
    m.insert("Sociology", "Social sciences");
    m.insert("Politics & Government", "Law & Jurisprudence");
    m.insert("Travel & Exploration", "History");
    m.insert("History", "History");
    m
});

// ---------------------------------------------------------------------------
// Compiled Regex Patterns
// ---------------------------------------------------------------------------

/// Validates standard Library of Congress code formats.
/// Matches `A` through `ZZZ` optionally followed by digits (`D501`, `F350.5`).
pub static RE_LC_CODE_VALID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z]{1,3}\d*(\.\d+)?$").unwrap());

/// Captures the alphabetic prefix (`A`, `AB`, `ABC`) from an LC code.
pub static RE_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([A-Z]{1,3})").unwrap());

/// Extracts numeric agent IDs from RDF `about` URLs (`agents/123`).
pub static RE_AGENT_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"agents/(\d+)").unwrap());

/// Captures `files/` or `dirs/` path segments for URL transformation.
pub static RE_FILES_DIRS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:files|dirs)/([^/]+)/(.+)").unwrap());

/// Removes the `Category:` prefix from bookshelf strings.
pub static RE_SHELF_CAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^Category:\s*").unwrap());

/// Matches MARC subfield markers (`$a`, `$b`, etc.) for cleaning.
pub static RE_MARC_SUBFIELD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$[a-zA-Z]\b").unwrap());

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms namespace table initialization.
    #[test]
    fn namespaces_contain_rdf() {
        assert!(NAMESPACES.contains_key("rdf"));
        assert_eq!(
            NAMESPACES.get("rdf").unwrap(),
            &"http://www.w3.org/1999/02/22-rdf-syntax-ns#"
        );
    }

    /// Confirms LC_MAP contains the `CB` entry.
    #[test]
    fn lc_map_has_history_entry() {
        assert!(LC_MAP.contains_key("CB"));
    }

    /// Validates the LC code regex against direct codes (`DA`) and
    /// numeric sub-codes (`F350.5`).
    #[test]
    fn regex_lc_valid_matches() {
        assert!(RE_LC_CODE_VALID.is_match("DA"));
        assert!(RE_LC_CODE_VALID.is_match("F350.5"));
    }
}
