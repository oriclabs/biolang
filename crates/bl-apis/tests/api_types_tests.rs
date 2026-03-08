//! External tests for all public types in bl-apis.
//!
//! Tests serde roundtrips, Display/Debug impls, and edge cases
//! for every public response type and the ApiError enum.

use bl_apis::error::ApiError;

// NCBI types
use bl_apis::ncbi::{DbInfo, GeneSummary, LinkResult, LinkSet, SearchResult};

// Ensembl types
use bl_apis::ensembl::{Gene, Ortholog, OrthologGene, Sequence, TranscriptConsequence, VepResult};

// UniProt types
use bl_apis::uniprot::{Feature, GoTerm, IdMapping, ProteinEntry};

// KEGG types
use bl_apis::kegg::{KeggEntry, KeggLink};

// STRING types
use bl_apis::string_db::{Enrichment, Interaction, StringProtein};

// GO types
use bl_apis::go::{Annotation, GoTermInfo, SlimResult};

// COSMIC types
use bl_apis::cosmic::{CensusGene, CosmicMutation};

// NCBI Datasets types
use bl_apis::ncbi_datasets::{DatasetGene, GenomeSummary, TaxonomyInfo};

// UCSC types
use bl_apis::ucsc::{Chromosome, Genome, Track};

// BioMart types
use bl_apis::biomart::{Attribute, Dataset, Filter};

// PDB types
use bl_apis::pdb::{PdbEntity, PdbEntry};

// Reactome types
use bl_apis::reactome::{Event, Pathway, ReactomeEntry};

// =========================================================================
// ApiError tests (moved from error.rs inline tests + new edge cases)
// =========================================================================

mod api_error {
    use super::*;

    #[test]
    fn test_http_error_display() {
        let err = ApiError::Http {
            status: 404,
            url: "https://example.com/api".into(),
            body: "Not Found".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("404"));
        assert!(msg.contains("example.com"));
        assert!(msg.contains("Not Found"));
    }

    #[test]
    fn test_network_error_display() {
        let err = ApiError::Network("connection refused".into());
        assert_eq!(err.to_string(), "network error: connection refused");
    }

    #[test]
    fn test_parse_error_display() {
        let err = ApiError::Parse {
            context: "NCBI esearch".into(),
            source: "missing idlist".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("NCBI esearch"));
        assert!(msg.contains("missing idlist"));
    }

    #[test]
    fn test_auth_error_display() {
        let err = ApiError::Auth("invalid API key".into());
        assert!(err.to_string().contains("auth error"));
    }

    #[test]
    fn test_rate_limit_with_retry() {
        let err = ApiError::RateLimit {
            retry_after: Some(60),
        };
        assert!(err.to_string().contains("60s"));
    }

    #[test]
    fn test_rate_limit_without_retry() {
        let err = ApiError::RateLimit { retry_after: None };
        assert_eq!(err.to_string(), "rate limited");
    }

    #[test]
    fn test_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(ApiError::Network("test".into()));
        assert!(err.to_string().contains("test"));
    }

    // --- New edge-case tests ---

    #[test]
    fn test_api_error_debug_impl() {
        let err = ApiError::Http {
            status: 500,
            url: "https://api.example.com".into(),
            body: "Internal Server Error".into(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("Http"));
        assert!(debug.contains("500"));
        assert!(debug.contains("Internal Server Error"));
    }

    #[test]
    fn test_http_error_400_bad_request() {
        let err = ApiError::Http {
            status: 400,
            url: "https://api.example.com/query".into(),
            body: "Bad Request".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("400"));
        assert!(msg.contains("Bad Request"));
    }

    #[test]
    fn test_http_error_401_unauthorized() {
        let err = ApiError::Http {
            status: 401,
            url: "https://api.example.com/secure".into(),
            body: "Unauthorized".into(),
        };
        assert!(err.to_string().contains("401"));
    }

    #[test]
    fn test_http_error_403_forbidden() {
        let err = ApiError::Http {
            status: 403,
            url: "https://api.example.com/admin".into(),
            body: "Forbidden".into(),
        };
        assert!(err.to_string().contains("403"));
    }

    #[test]
    fn test_http_error_500_server() {
        let err = ApiError::Http {
            status: 500,
            url: "https://api.example.com".into(),
            body: "Internal Server Error".into(),
        };
        assert!(err.to_string().contains("500"));
    }

    #[test]
    fn test_http_error_503_unavailable() {
        let err = ApiError::Http {
            status: 503,
            url: "https://api.example.com".into(),
            body: "Service Unavailable".into(),
        };
        assert!(err.to_string().contains("503"));
        assert!(err.to_string().contains("Service Unavailable"));
    }

    #[test]
    fn test_rate_limit_very_large_retry_after() {
        let err = ApiError::RateLimit {
            retry_after: Some(86400),
        };
        assert!(err.to_string().contains("86400s"));
    }

    #[test]
    fn test_parse_error_empty_context() {
        let err = ApiError::Parse {
            context: "".into(),
            source: "something broke".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("parse error in"));
        assert!(msg.contains("something broke"));
    }

    #[test]
    fn test_network_error_debug() {
        let err = ApiError::Network("DNS resolution failed".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Network"));
        assert!(debug.contains("DNS resolution failed"));
    }

    #[test]
    fn test_auth_error_debug() {
        let err = ApiError::Auth("expired token".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Auth"));
        assert!(debug.contains("expired token"));
    }

    #[test]
    fn test_rate_limit_debug() {
        let err = ApiError::RateLimit {
            retry_after: Some(30),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("RateLimit"));
        assert!(debug.contains("30"));
    }
}

// =========================================================================
// Serde roundtrip tests for all public types
// =========================================================================

mod serde_roundtrips {
    use super::*;

    // --- NCBI ---

    #[test]
    fn search_result_roundtrip() {
        let sr = SearchResult {
            ids: vec!["672".into(), "675".into()],
            count: 2,
            webenv: Some("MCID_abc".into()),
            query_key: Some("1".into()),
        };
        let json = serde_json::to_value(&sr).unwrap();
        let back: SearchResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.ids, vec!["672", "675"]);
        assert_eq!(back.count, 2);
        assert_eq!(back.webenv.as_deref(), Some("MCID_abc"));
        assert_eq!(back.query_key.as_deref(), Some("1"));
    }

    #[test]
    fn search_result_no_optional_fields() {
        let sr = SearchResult {
            ids: vec![],
            count: 0,
            webenv: None,
            query_key: None,
        };
        let json = serde_json::to_value(&sr).unwrap();
        let back: SearchResult = serde_json::from_value(json).unwrap();
        assert!(back.ids.is_empty());
        assert!(back.webenv.is_none());
    }

    #[test]
    fn gene_summary_roundtrip() {
        let gs = GeneSummary {
            id: "672".into(),
            name: "BRCA1".into(),
            symbol: "BRCA1".into(),
            description: "BRCA1 DNA repair associated".into(),
            organism: "Homo sapiens".into(),
            chromosome: "17".into(),
            location: "17q21.31".into(),
            summary: "Tumor suppressor".into(),
        };
        let json = serde_json::to_value(&gs).unwrap();
        let back: GeneSummary = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "672");
        assert_eq!(back.symbol, "BRCA1");
        assert_eq!(back.organism, "Homo sapiens");
    }

    #[test]
    fn gene_summary_defaults() {
        let json = serde_json::json!({"id": "999"});
        let gs: GeneSummary = serde_json::from_value(json).unwrap();
        assert_eq!(gs.id, "999");
        assert_eq!(gs.name, "");
        assert_eq!(gs.symbol, "");
    }

    #[test]
    fn db_info_roundtrip() {
        let info = DbInfo {
            db_name: "gene".into(),
            count: 50000,
            description: "Gene database".into(),
        };
        let json = serde_json::to_value(&info).unwrap();
        let back: DbInfo = serde_json::from_value(json).unwrap();
        assert_eq!(back.db_name, "gene");
        assert_eq!(back.count, 50000);
    }

    #[test]
    fn link_result_roundtrip() {
        let lr = LinkResult {
            links: vec![
                LinkSet {
                    dbto: "protein".into(),
                    ids: vec!["NP_001".into(), "NP_002".into()],
                },
                LinkSet {
                    dbto: "pubmed".into(),
                    ids: vec!["12345".into()],
                },
            ],
        };
        let json = serde_json::to_value(&lr).unwrap();
        let back: LinkResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.links.len(), 2);
        assert_eq!(back.links[0].dbto, "protein");
        assert_eq!(back.links[1].ids, vec!["12345"]);
    }

    #[test]
    fn link_result_empty_links() {
        let lr = LinkResult { links: vec![] };
        let json = serde_json::to_value(&lr).unwrap();
        let back: LinkResult = serde_json::from_value(json).unwrap();
        assert!(back.links.is_empty());
    }

    // --- Ensembl ---

    #[test]
    fn ensembl_gene_roundtrip() {
        let gene = Gene {
            id: "ENSG00000012048".into(),
            symbol: "BRCA1".into(),
            description: "BRCA1 DNA repair associated".into(),
            species: "homo_sapiens".into(),
            biotype: "protein_coding".into(),
            start: 43044295,
            end: 43170245,
            strand: -1,
            chromosome: "17".into(),
        };
        let json = serde_json::to_value(&gene).unwrap();
        let back: Gene = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "ENSG00000012048");
        assert_eq!(back.strand, -1);
        assert_eq!(back.chromosome, "17");
    }

    #[test]
    fn ensembl_gene_defaults() {
        let json = serde_json::json!({"id": "ENSG00000000001"});
        let gene: Gene = serde_json::from_value(json).unwrap();
        assert_eq!(gene.id, "ENSG00000000001");
        assert_eq!(gene.symbol, "");
        assert_eq!(gene.start, 0);
        assert_eq!(gene.strand, 0);
    }

    #[test]
    fn sequence_roundtrip() {
        let seq = Sequence {
            id: "ENST00000357654".into(),
            seq: "ATCGATCG".into(),
            molecule: "cdna".into(),
        };
        let json = serde_json::to_value(&seq).unwrap();
        let back: Sequence = serde_json::from_value(json).unwrap();
        assert_eq!(back.seq, "ATCGATCG");
    }

    #[test]
    fn vep_result_roundtrip() {
        let vep = VepResult {
            allele_string: "C/T".into(),
            most_severe_consequence: "missense_variant".into(),
            transcript_consequences: vec![TranscriptConsequence {
                gene_id: "ENSG1".into(),
                transcript_id: "ENST1".into(),
                consequence_terms: vec!["missense_variant".into()],
                impact: "MODERATE".into(),
                biotype: "protein_coding".into(),
            }],
        };
        let json = serde_json::to_value(&vep).unwrap();
        let back: VepResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.allele_string, "C/T");
        assert_eq!(back.transcript_consequences.len(), 1);
        assert_eq!(back.transcript_consequences[0].impact, "MODERATE");
    }

    #[test]
    fn vep_result_no_consequences() {
        let vep = VepResult {
            allele_string: "A/G".into(),
            most_severe_consequence: "intergenic_variant".into(),
            transcript_consequences: vec![],
        };
        let json = serde_json::to_value(&vep).unwrap();
        let back: VepResult = serde_json::from_value(json).unwrap();
        assert!(back.transcript_consequences.is_empty());
    }

    #[test]
    fn ortholog_roundtrip() {
        let orth = Ortholog {
            source: OrthologGene {
                id: "ENSG00000012048".into(),
                species: "homo_sapiens".into(),
            },
            target: OrthologGene {
                id: "ENSMUSG00000017146".into(),
                species: "mus_musculus".into(),
            },
            type_: "ortholog_one2one".into(),
        };
        let json = serde_json::to_value(&orth).unwrap();
        let back: Ortholog = serde_json::from_value(json).unwrap();
        assert_eq!(back.source.species, "homo_sapiens");
        assert_eq!(back.target.species, "mus_musculus");
    }

    // --- UniProt ---

    #[test]
    fn protein_entry_roundtrip() {
        let entry = ProteinEntry {
            accession: "P38398".into(),
            name: "Breast cancer type 1 susceptibility protein".into(),
            organism: "Homo sapiens".into(),
            sequence_length: 1863,
            gene_names: vec!["BRCA1".into()],
            function: "E3 ubiquitin-protein ligase".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let back: ProteinEntry = serde_json::from_value(json).unwrap();
        assert_eq!(back.accession, "P38398");
        assert_eq!(back.sequence_length, 1863);
        assert_eq!(back.gene_names, vec!["BRCA1"]);
    }

    #[test]
    fn protein_entry_empty_gene_names() {
        let entry = ProteinEntry {
            accession: "A0A000".into(),
            name: "".into(),
            organism: "".into(),
            sequence_length: 0,
            gene_names: vec![],
            function: "".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let back: ProteinEntry = serde_json::from_value(json).unwrap();
        assert!(back.gene_names.is_empty());
    }

    #[test]
    fn protein_entry_multiple_gene_names() {
        let entry = ProteinEntry {
            accession: "P04637".into(),
            name: "Cellular tumor antigen p53".into(),
            organism: "Homo sapiens".into(),
            sequence_length: 393,
            gene_names: vec!["TP53".into(), "P53".into()],
            function: "Acts as a tumor suppressor".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let back: ProteinEntry = serde_json::from_value(json).unwrap();
        assert_eq!(back.gene_names.len(), 2);
        assert_eq!(back.gene_names[0], "TP53");
        assert_eq!(back.gene_names[1], "P53");
    }

    #[test]
    fn feature_roundtrip() {
        let feat = Feature {
            type_: "Domain".into(),
            location: "10..50".into(),
            description: "RING-type".into(),
        };
        let json = serde_json::to_value(&feat).unwrap();
        let back: Feature = serde_json::from_value(json).unwrap();
        assert_eq!(back.description, "RING-type");
    }

    #[test]
    fn go_term_roundtrip() {
        let term = GoTerm {
            id: "GO:0005634".into(),
            term: "nucleus".into(),
            aspect: "C".into(),
        };
        let json = serde_json::to_value(&term).unwrap();
        let back: GoTerm = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "GO:0005634");
    }

    #[test]
    fn id_mapping_roundtrip() {
        let mapping = IdMapping {
            from: "P38398".into(),
            to: "672".into(),
        };
        let json = serde_json::to_value(&mapping).unwrap();
        let back: IdMapping = serde_json::from_value(json).unwrap();
        assert_eq!(back.from, "P38398");
        assert_eq!(back.to, "672");
    }

    // --- KEGG ---

    #[test]
    fn kegg_entry_roundtrip() {
        let entry = KeggEntry {
            id: "hsa:4609".into(),
            description: "MYC proto-oncogene".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let back: KeggEntry = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "hsa:4609");
        assert_eq!(back.description, "MYC proto-oncogene");
    }

    #[test]
    fn kegg_link_roundtrip() {
        let link = KeggLink {
            source: "hsa:10458".into(),
            target: "path:hsa04010".into(),
        };
        let json = serde_json::to_value(&link).unwrap();
        let back: KeggLink = serde_json::from_value(json).unwrap();
        assert_eq!(back.source, "hsa:10458");
        assert_eq!(back.target, "path:hsa04010");
    }

    // --- STRING ---

    #[test]
    fn interaction_roundtrip() {
        let inter = Interaction {
            protein_a: "TP53".into(),
            protein_b: "MDM2".into(),
            score: 0.999,
            nscore: 0.0,
            fscore: 0.0,
            pscore: 0.0,
            ascore: 0.0,
            escore: 0.943,
            dscore: 0.0,
            tscore: 0.989,
        };
        let json = serde_json::to_value(&inter).unwrap();
        let back: Interaction = serde_json::from_value(json).unwrap();
        assert_eq!(back.protein_a, "TP53");
        assert!((back.score - 0.999).abs() < 0.001);
    }

    #[test]
    fn interaction_all_zeros() {
        let inter = Interaction {
            protein_a: "".into(),
            protein_b: "".into(),
            score: 0.0,
            nscore: 0.0,
            fscore: 0.0,
            pscore: 0.0,
            ascore: 0.0,
            escore: 0.0,
            dscore: 0.0,
            tscore: 0.0,
        };
        let json = serde_json::to_value(&inter).unwrap();
        let back: Interaction = serde_json::from_value(json).unwrap();
        assert_eq!(back.score, 0.0);
    }

    #[test]
    fn enrichment_roundtrip() {
        let enr = Enrichment {
            category: "Process".into(),
            term: "GO:0006915".into(),
            description: "apoptotic process".into(),
            gene_count: 5,
            p_value: 1.2e-10,
            fdr: 3.4e-8,
        };
        let json = serde_json::to_value(&enr).unwrap();
        let back: Enrichment = serde_json::from_value(json).unwrap();
        assert_eq!(back.category, "Process");
        assert_eq!(back.gene_count, 5);
    }

    #[test]
    fn string_protein_roundtrip() {
        let prot = StringProtein {
            preferred_name: "TP53".into(),
            string_id: "9606.ENSP00000269305".into(),
            annotation: "Cellular tumor antigen p53".into(),
        };
        let json = serde_json::to_value(&prot).unwrap();
        let back: StringProtein = serde_json::from_value(json).unwrap();
        assert_eq!(back.preferred_name, "TP53");
        assert_eq!(back.string_id, "9606.ENSP00000269305");
    }

    // --- GO ---

    #[test]
    fn go_term_info_roundtrip() {
        let term = GoTermInfo {
            id: "GO:0008150".into(),
            name: "biological_process".into(),
            aspect: "biological_process".into(),
            definition: "A biological process represents a specific objective...".into(),
            is_obsolete: false,
        };
        let json = serde_json::to_value(&term).unwrap();
        let back: GoTermInfo = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "GO:0008150");
        assert!(!back.is_obsolete);
    }

    #[test]
    fn go_term_info_obsolete() {
        let term = GoTermInfo {
            id: "GO:0000001".into(),
            name: "old term".into(),
            aspect: "".into(),
            definition: "".into(),
            is_obsolete: true,
        };
        let json = serde_json::to_value(&term).unwrap();
        let back: GoTermInfo = serde_json::from_value(json).unwrap();
        assert!(back.is_obsolete);
    }

    #[test]
    fn annotation_roundtrip() {
        let ann = Annotation {
            go_id: "GO:0005634".into(),
            go_name: "nucleus".into(),
            aspect: "cellular_component".into(),
            evidence: "IDA".into(),
            qualifier: "located_in".into(),
            gene_product_id: "UniProtKB:P38398".into(),
            symbol: "BRCA1".into(),
        };
        let json = serde_json::to_value(&ann).unwrap();
        let back: Annotation = serde_json::from_value(json).unwrap();
        assert_eq!(back.go_id, "GO:0005634");
        assert_eq!(back.evidence, "IDA");
    }

    #[test]
    fn slim_result_roundtrip() {
        let slim = SlimResult {
            go_id: "GO:0008150".into(),
            mapped_to: vec!["GO:0006915".into(), "GO:0007049".into()],
        };
        let json = serde_json::to_value(&slim).unwrap();
        let back: SlimResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.mapped_to.len(), 2);
    }

    // --- COSMIC ---

    #[test]
    fn cosmic_mutation_roundtrip() {
        let m = CosmicMutation {
            id: "COSM476".into(),
            gene: "BRAF".into(),
            cds: "c.1799T>A".into(),
            aa: "p.V600E".into(),
            primary_site: "thyroid".into(),
            primary_histology: "carcinoma".into(),
            mutation_type: "Substitution - Missense".into(),
            count: 48238,
        };
        let json = serde_json::to_value(&m).unwrap();
        let back: CosmicMutation = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "COSM476");
        assert_eq!(back.count, 48238);
    }

    #[test]
    fn cosmic_mutation_defaults() {
        let json = serde_json::json!({});
        let m: CosmicMutation = serde_json::from_value(json).unwrap();
        assert_eq!(m.id, "");
        assert_eq!(m.gene, "");
        assert_eq!(m.count, 0);
    }

    #[test]
    fn census_gene_roundtrip() {
        let g = CensusGene {
            gene_symbol: "TP53".into(),
            name: "Tumor protein p53".into(),
            role_in_cancer: "TSG".into(),
            tier: 1,
            hallmark: true,
            somatic: true,
            germline: true,
        };
        let json = serde_json::to_value(&g).unwrap();
        let back: CensusGene = serde_json::from_value(json).unwrap();
        assert_eq!(back.gene_symbol, "TP53");
        assert_eq!(back.tier, 1);
        assert!(back.hallmark);
    }

    #[test]
    fn census_gene_defaults() {
        let json = serde_json::json!({});
        let g: CensusGene = serde_json::from_value(json).unwrap();
        assert_eq!(g.gene_symbol, "");
        assert_eq!(g.tier, 0);
        assert!(!g.hallmark);
        assert!(!g.somatic);
    }

    // --- NCBI Datasets ---

    #[test]
    fn dataset_gene_roundtrip() {
        let gene = DatasetGene {
            gene_id: "672".into(),
            symbol: "BRCA1".into(),
            description: "BRCA1 DNA repair associated".into(),
            taxname: "Homo sapiens".into(),
            chromosome: "17".into(),
            gene_type: "PROTEIN_CODING".into(),
            common_name: "human".into(),
        };
        let json = serde_json::to_value(&gene).unwrap();
        let back: DatasetGene = serde_json::from_value(json).unwrap();
        assert_eq!(back.gene_id, "672");
        assert_eq!(back.chromosome, "17");
    }

    #[test]
    fn taxonomy_info_roundtrip() {
        let tax = TaxonomyInfo {
            tax_id: "9606".into(),
            organism_name: "Homo sapiens".into(),
            common_name: "human".into(),
            lineage: vec!["Eukaryota".into(), "Metazoa".into(), "Chordata".into()],
            rank: "species".into(),
        };
        let json = serde_json::to_value(&tax).unwrap();
        let back: TaxonomyInfo = serde_json::from_value(json).unwrap();
        assert_eq!(back.tax_id, "9606");
        assert_eq!(back.lineage.len(), 3);
    }

    #[test]
    fn taxonomy_info_empty_lineage() {
        let tax = TaxonomyInfo {
            tax_id: "1".into(),
            organism_name: "root".into(),
            common_name: "".into(),
            lineage: vec![],
            rank: "no rank".into(),
        };
        let json = serde_json::to_value(&tax).unwrap();
        let back: TaxonomyInfo = serde_json::from_value(json).unwrap();
        assert!(back.lineage.is_empty());
    }

    #[test]
    fn genome_summary_roundtrip() {
        let gs = GenomeSummary {
            accession: "GCF_000001405.40".into(),
            organism_name: "Homo sapiens".into(),
            assembly_level: "Chromosome".into(),
            assembly_name: "GRCh38.p14".into(),
            total_sequence_length: 3_088_286_401,
            gc_percent: 40.9,
        };
        let json = serde_json::to_value(&gs).unwrap();
        let back: GenomeSummary = serde_json::from_value(json).unwrap();
        assert_eq!(back.accession, "GCF_000001405.40");
        assert_eq!(back.total_sequence_length, 3_088_286_401);
    }

    // --- UCSC ---

    #[test]
    fn genome_roundtrip() {
        let g = Genome {
            name: "hg38".into(),
            description: "Human Dec. 2013 (GRCh38/hg38)".into(),
            organism: "Human".into(),
        };
        let json = serde_json::to_value(&g).unwrap();
        let back: Genome = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "hg38");
    }

    #[test]
    fn track_roundtrip() {
        let t = Track {
            name: "knownGene".into(),
            short_label: "GENCODE V44".into(),
            long_label: "GENCODE V44 comprehensive gene annotations".into(),
            type_: "genePred".into(),
        };
        let json = serde_json::to_value(&t).unwrap();
        let back: Track = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "knownGene");
    }

    #[test]
    fn chromosome_roundtrip() {
        let c = Chromosome {
            chrom: "chr1".into(),
            size: 248956422,
        };
        let json = serde_json::to_value(&c).unwrap();
        let back: Chromosome = serde_json::from_value(json).unwrap();
        assert_eq!(back.chrom, "chr1");
        assert_eq!(back.size, 248956422);
    }

    // --- BioMart ---

    #[test]
    fn dataset_roundtrip() {
        let ds = Dataset {
            name: "hsapiens_gene_ensembl".into(),
            display_name: "Human genes (GRCh38.p14)".into(),
        };
        let json = serde_json::to_value(&ds).unwrap();
        let back: Dataset = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "hsapiens_gene_ensembl");
    }

    #[test]
    fn attribute_roundtrip() {
        let a = Attribute {
            name: "ensembl_gene_id".into(),
            display_name: "Gene stable ID".into(),
        };
        let json = serde_json::to_value(&a).unwrap();
        let back: Attribute = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "ensembl_gene_id");
    }

    #[test]
    fn filter_roundtrip() {
        let f = Filter {
            name: "chromosome_name".into(),
            display_name: "Chromosome/scaffold name".into(),
        };
        let json = serde_json::to_value(&f).unwrap();
        let back: Filter = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "chromosome_name");
    }

    // --- PDB ---

    #[test]
    fn pdb_entry_roundtrip() {
        let entry = PdbEntry {
            id: "1TUP".into(),
            title: "Crystal structure of p53".into(),
            method: "X-RAY DIFFRACTION".into(),
            resolution: Some(2.2),
            release_date: "1995-01-31".into(),
            organism: "Homo sapiens".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let back: PdbEntry = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "1TUP");
        assert!((back.resolution.unwrap() - 2.2).abs() < 0.01);
    }

    #[test]
    fn pdb_entry_no_resolution() {
        let entry = PdbEntry {
            id: "1ABC".into(),
            title: "NMR structure".into(),
            method: "SOLUTION NMR".into(),
            resolution: None,
            release_date: "2020-01-01".into(),
            organism: "".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let back: PdbEntry = serde_json::from_value(json).unwrap();
        assert!(back.resolution.is_none());
    }

    #[test]
    fn pdb_entity_roundtrip() {
        let entity = PdbEntity {
            entity_id: 1,
            description: "p53 tumor suppressor".into(),
            entity_type: "polymer".into(),
            sequence: "MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPS".into(),
        };
        let json = serde_json::to_value(&entity).unwrap();
        let back: PdbEntity = serde_json::from_value(json).unwrap();
        assert_eq!(back.entity_id, 1);
        assert!(back.sequence.starts_with("MEEPQSDP"));
    }

    // --- Reactome ---

    #[test]
    fn reactome_entry_roundtrip() {
        let entry = ReactomeEntry {
            id: "R-HSA-109581".into(),
            name: "Apoptosis".into(),
            schema_class: "Pathway".into(),
            species: "Homo sapiens".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let back: ReactomeEntry = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "R-HSA-109581");
        assert_eq!(back.schema_class, "Pathway");
    }

    #[test]
    fn pathway_roundtrip() {
        let pw = Pathway {
            id: "R-HSA-109581".into(),
            name: "Apoptosis".into(),
            species: "Homo sapiens".into(),
            is_disease: false,
            is_inferred: false,
        };
        let json = serde_json::to_value(&pw).unwrap();
        let back: Pathway = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "R-HSA-109581");
        assert!(!back.is_disease);
    }

    #[test]
    fn pathway_disease() {
        let pw = Pathway {
            id: "R-HSA-123".into(),
            name: "Disease pathway".into(),
            species: "Homo sapiens".into(),
            is_disease: true,
            is_inferred: true,
        };
        let json = serde_json::to_value(&pw).unwrap();
        let back: Pathway = serde_json::from_value(json).unwrap();
        assert!(back.is_disease);
        assert!(back.is_inferred);
    }

    #[test]
    fn event_roundtrip() {
        let ev = Event {
            id: "R-HSA-69278".into(),
            name: "Cell Cycle, Mitotic".into(),
            schema_class: "TopLevelPathway".into(),
        };
        let json = serde_json::to_value(&ev).unwrap();
        let back: Event = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "R-HSA-69278");
        assert_eq!(back.schema_class, "TopLevelPathway");
    }

    // --- DocSummary (special: uses #[serde(flatten)]) ---

    #[test]
    fn doc_summary_roundtrip() {
        use bl_apis::ncbi::DocSummary;
        let mut fields = serde_json::Map::new();
        fields.insert("name".into(), serde_json::json!("BRCA1"));
        fields.insert("chromosome".into(), serde_json::json!("17"));

        let doc = DocSummary {
            uid: "672".into(),
            fields,
        };
        let json = serde_json::to_value(&doc).unwrap();
        let back: DocSummary = serde_json::from_value(json).unwrap();
        assert_eq!(back.uid, "672");
        assert_eq!(back.fields.get("name").unwrap(), "BRCA1");
    }
}
