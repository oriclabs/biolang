// BioGist Core — Shared detection, rendering, and export logic
// No browser extension APIs (chrome.*, browser.*) — works in extension AND PWA
// Usage: const core = window.BioGistCore;

(function() {
  "use strict";

  // ── Gene Symbol Set ────────────────────────────────────────────────
  // Use full HGNC set if loaded (44,959 symbols), else fallback to curated set
  // HGNC loaded via <script src="hgnc-symbols.js"> before this file

  const GENE_SYMBOLS_FALLBACK = new Set([
    "TP53","BRCA1","BRCA2","PTEN","RB1","APC","VHL","KRAS","NRAS","BRAF",
    "PIK3CA","EGFR","ERBB2","HER2","ALK","RET","MET","ROS1","MYC","MYCN",
    "CDKN2A","CDK4","CDK6","MDM2","ATM","ATR","CHEK2","PALB2","RAD51",
    "MLH1","MSH2","MSH6","PMS2","BRIP1","JAK2","FLT3","KIT","ABL1","BCR",
    "NOTCH1","CTNNB1","SMAD4","STK11","NF1","NF2","TSC1","TSC2","BAP1",
    "IDH1","IDH2","DNMT3A","TET2","NPM1","RUNX1","CEBPA","WT1","GATA2",
    "TERT","FGFR1","FGFR2","FGFR3","PDGFRA","MAP2K1","MTOR","AKT1",
    "EZH2","ARID1A","PBRM1","SETD2","KDM6A","KMT2D","CREBBP",
    "BCL2","BCL6","MYD88","CARD11","BTK","SOX2","FOXL2","DICER1",
    "CFTR","HTT","DMD","SMN1","GBA","LRRK2","SNCA","APP","PSEN1","APOE",
    "HBB","G6PD","F5","MTHFR","CYP2D6","CYP2C19","DPYD","UGT1A1","ACE2",
    "GAPDH","ACTB","VEGFA","TNF","IL6","TGFB1","SHH","WNT1","STAT3",
    "CCND1","CCNE1","MDM4","POLE","POLD1","MUTYH","FANCA","FANCC",
    "NBN","MRE11","RAD51C","RAD51D","BARD1","SMARCB1",
    "CD79A","CD79B","PLCG2","IL2","IL7","IFNG","PDGFB","FGF2",
    "SMAD2","SMAD3","BMP4","WNT3A","GLI1","PTCH1","SMO","SUFU",
    "TMPRSS2","FMR1","SMN2","PKD1","PKD2","SERPINA1","VKORC1",
    "TPMT","NUDT15","CYP3A4","SLCO1B1","ABCB1","NAT2",
    "PSEN2","MAPT","GRN","C9orf72","SOD1","FUS","TARDBP",
    "HBA1","HBA2","GATA1","SPI1","PAX5","EBF1","IKZF1"
  ]);

  // Use full HGNC if loaded, otherwise fallback
  const GENE_SYMBOLS = (typeof window !== "undefined" && window.HGNC_SYMBOLS && window.HGNC_SYMBOLS.size > 1000)
    ? window.HGNC_SYMBOLS
    : GENE_SYMBOLS_FALLBACK;

  // Expanded exclusion list — critical for 44K gene symbols to avoid false positives
  // Many HGNC symbols are common English words (e.g., IMPACT, REST, TANK, CAST, WARS)
  const EXCLUDE = new Set([
    // Common English words that are also gene symbols
    "THE","AND","FOR","NOT","WITH","FROM","BUT","ALL","ARE","WAS","WERE",
    "HAS","HAD","BEEN","HAVE","THAT","THIS","WILL","CAN","MAY","SET",
    "MAP","LET","RUN","USE","AGE","END","TOP","ACE","HER","HIS",
    "OUR","WHO","HOW","WHY","DID","GOT","PUT","SAY","GET","SEE",
    "OLD","NEW","NOW","WAY","DAY","TWO","RNA","DNA",
    // Additional common words that conflict with HGNC symbols
    "REST","CAST","TANK","WARS","IMPACT","CHANCE","CLOCK","CHIP",
    "CARD","COPE","COPE","CASK","CAPS","CLIP","COIL","COPE",
    "DISC","DOCK","DOOR","DRAW","DROP","DUST","EDGE","FACE",
    "FAST","FATE","FEAR","FEED","FIRE","FISH","FLAG","FLAP",
    "FLIP","FLOW","FOLD","FOOD","FOOL","FORK","FORM","FRET",
    "FUEL","FUSE","GAIN","GAIT","GALE","GAME","GAPS","GAZE",
    "GENE","GIFT","GLOW","GLUE","GOAL","GOLD","GRAB","GRIP",
    "GROW","GULF","GUST","HALO","HALT","HAND","HANG","HARM",
    "HAZE","HEAP","HEAT","HEED","HELP","HERE","HIDE","HIGH",
    "HINT","HOLD","HOLE","HOME","HOOK","HOPE","HOST","HUNT",
    "ICON","IDEA","IRON","ITEM","JADE","JUNK","JUST","KEEN",
    "KEEP","KICK","KIDS","KIND","KING","KISS","KNIT","KNOW",
    "LACK","LAME","LAMP","LAND","LANE","LAST","LATE","LEAD",
    "LEAN","LEAP","LEFT","LENS","LESS","LIEN","LIFE","LIFT",
    "LIKE","LIMB","LIME","LINE","LINK","LIST","LIVE","LOAD",
    "LOCK","LONE","LONG","LOOK","LOOP","LOSS","LOST","LOVE",
    "LUCK","LUMP","LUNG","LURE","MADE","MAIN","MAKE","MALE",
    "MANY","MARK","MARS","MASH","MASK","MASS","MATE","MATH",
    "MAZE","MEAL","MEAN","MEET","MELT","MEMO","MEND","MESH",
    "MILD","MIND","MINE","MISS","MODE","MOLD","MOOD","MOON",
    "MORE","MOST","MOVE","MUCH","MUST","MYTH","NAME","NEAR",
    "NEAT","NECK","NEED","NEST","NEXT","NICE","NODE","NONE",
    "NORM","NOSE","NOTE","NULL","ODDS","ONCE","ONLY","OPEN",
    "OVER","PACE","PACK","PAGE","PAID","PAIR","PALE","PALM",
    "PANEL","PARK","PART","PASS","PAST","PATH","PEAK","PICK",
    "PILE","PINE","PIPE","PLAN","PLAY","PLOT","PLUG","PLUS",
    "POLL","POND","POOL","POOR","POSE","POST","POUR","PRAY",
    "PULL","PUMP","PURE","PUSH","RACE","RACK","RAGE","RAID",
    "RAIL","RAIN","RANK","RARE","RATE","READ","REAL","REAR",
    "RELY","RENT","RIDE","RING","RISE","RISK","ROAD","ROCK",
    "ROLE","ROLL","ROOF","ROOM","ROOT","ROPE","ROSE","RULE",
    "RUSH","RUST","SAFE","SAGE","SAIL","SAKE","SALE","SALT",
    "SAME","SAND","SAVE","SCAN","SEAL","SEAT","SEED","SEEK",
    "SEEM","SELF","SELL","SEND","SHED","SHIP","SHOP","SHOT",
    "SHOW","SHUT","SICK","SIDE","SIGN","SILK","SINK","SITE",
    "SIZE","SKIN","SKIP","SLAM","SLIP","SLOT","SLOW","SNAP",
    "SNOW","SOAK","SOFT","SOIL","SOLE","SOME","SONG","SOON",
    "SORT","SOUL","SPAN","SPEC","SPIN","SPOT","STAR","STAY",
    "STEM","STEP","STIR","STOP","SUCH","SUIT","SURE","SURF",
    "SWAP","SWIM","TACT","TAIL","TAKE","TALE","TALK","TALL",
    "TAPE","TASK","TEAM","TEAR","TELL","TEND","TERM","TEST",
    "TEXT","THAN","THEM","THEN","THEY","THIN","TIDE","TILE",
    "TILL","TIME","TINY","TIRE","TOLL","TONE","TOOL","TORN",
    "TOUR","TOWN","TRAP","TRAY","TREE","TREK","TRIM","TRIO",
    "TRIP","TRUE","TUBE","TUCK","TUNE","TURN","TWIN","TYPE",
    "UNIT","UPON","URGE","USED","USER","VAST","VARY","VERB",
    "VERY","VIEW","VINE","VOID","VOTE","WADE","WAGE","WAIT",
    "WAKE","WALK","WALL","WANT","WARD","WARM","WARN","WASH",
    "WAVE","WEAK","WEAR","WEEK","WELL","WENT","WEST","WHAT",
    "WHEN","WIDE","WIFE","WILD","WIND","WINE","WING","WIRE",
    "WISE","WISH","WOOD","WOOL","WORD","WORE","WORK","WORM",
    "WORN","WRAP","YARD","YEAR","ZERO","ZONE",
    // 3-letter common words
    "ADD","AIR","AIM","ARC","ARM","ART","ASK","ATE","BAD","BAG",
    "BAR","BAT","BED","BIG","BIT","BOW","BOX","BOY","BUS","BUY",
    "CAB","CAN","CAP","CAR","CUT","DIG","DOG","DRY","DUE","EAR",
    "EAT","EGG","ERA","EVE","EYE","FAN","FAR","FAT","FED","FEW",
    "FIG","FIT","FLY","FOG","FUN","FUR","GAP","GAS","GEL","GIG",
    "GUM","GUN","GUT","GUY","HAD","HAM","HAT","HIT","HOG","HOP",
    "HOT","HUB","HUG","ICE","ILL","INK","INN","ION","JAM","JAR",
    "JAW","JET","JOB","JOG","JOY","KEY","KID","KIN","KIT","LAB",
    "LAP","LAW","LAY","LED","LEG","LID","LIE","LIP","LIT","LOG",
    "LOT","LOW","MAD","MAN","MAT","MEN","MET","MID","MIX","MOB",
    "MOP","MUD","MUG","NAP","NET","NIT","NOD","NOR","NUN","NUT",
    "OAK","OAR","OAT","ODD","OIL","OPT","ORB","ORE","OWE","OWL",
    "OWN","PAD","PAN","PAT","PAW","PAY","PEA","PEG","PEN","PER",
    "PET","PIE","PIG","PIN","PIT","PLY","POD","POP","POT","POW",
    "PRE","PRO","PUB","PUG","PUN","PUP","RAG","RAM","RAN","RAP",
    "RAT","RAW","RAY","RED","REF","RIB","RID","RIG","RIM","RIP",
    "ROB","ROD","ROT","ROW","RUB","RUG","SAD","SAP","SAT","SAW",
    "SHE","SHY","SIP","SIS","SIT","SIX","SKI","SKY","SLY","SOB",
    "SOD","SON","SOP","SOT","SOW","SOY","SPA","SPY","STY","SUB",
    "SUM","SUN","SUP","TAB","TAD","TAG","TAN","TAP","TAR","TAT",
    "TAX","TEA","TEN","THE","TIE","TIN","TIP","TOE","TON","TOO",
    "TOW","TOY","TUB","TUG","VAN","VAT","VET","VIA","VOW","WAD",
    "WAR","WAX","WEB","WED","WET","WHO","WIG","WIN","WIT","WOE",
    "WOK","WON","WOO","WOW","YAM","YAP","YAW","YES","YET","YEW",
    // File formats & acronyms
    "PDF","CSV","TSV","TXT","PNG","JPG","SVG","GIF","URL","API","SQL","XML",
    "HTTP","HTML","CSS","JSON","YAML","TOML","WASM",
    // 2-letter abbreviations that clash with gene symbols
    "CP","KL","HP","PC","HR","PM","AM","BP","OS","DR","MR","MS","ML",
    "CI","SD","SE","OR","GO","QC","IO","ID","UI","PH","PR","KB","MB","GB"
  ]);

  // ── Entity Type Metadata ───────────────────────────────────────────

  const TYPE_META = {
    gene:         { icon: "\u{1F9EC}", badge: "gene",         cls: "badge-gene",         label: "Genes" },
    variant:      { icon: "\u{1F52C}", badge: "variant",      cls: "badge-variant",      label: "Variants" },
    accession:    { icon: "\u{1F4CA}", badge: "accession",    cls: "badge-accession",    label: "Accessions" },
    method:       { icon: "\u{1F527}", badge: "method",       cls: "badge-method",       label: "Methods & Tools" },
    genome_build: { icon: "\u{1F9E9}", badge: "build",        cls: "badge-genome_build", label: "Genome Build" },
    sample_size:  { icon: "\u{1F4CF}", badge: "size",         cls: "badge-sample_size",  label: "Sample Sizes" },
    stat_method:  { icon: "\u{1F4C8}", badge: "stats",        cls: "badge-stat_method",  label: "Statistics" },
    platform:     { icon: "\u2699",    badge: "platform",     cls: "badge-platform",     label: "Sequencing Platform" },
    cell_line:    { icon: "\u{1F9EB}", badge: "cell",         cls: "badge-cell_line",    label: "Cell Lines" },
    tissue:       { icon: "\u{1FA78}", badge: "tissue",       cls: "badge-tissue",       label: "Tissues" },
    drug:         { icon: "\u{1F48A}", badge: "drug",         cls: "badge-drug",         label: "Drugs" },
    finding:      { icon: "\u{1F4A1}", badge: "finding",      cls: "badge-finding",      label: "Key Findings" },
    clinical_trial:{ icon: "\u{1F3E5}", badge: "trial",       cls: "badge-trial",        label: "Clinical Trials" },
    funding:      { icon: "\u{1F4B0}", badge: "grant",        cls: "badge-funding",      label: "Funding" },
    repository:   { icon: "\u{1F4E6}", badge: "repo",         cls: "badge-repository",   label: "Repositories" },
    p_value:      { icon: "\u{1F4CA}", badge: "p-val",        cls: "badge-p_value",      label: "P-values" },
    file:         { icon: "\u{1F4C1}", badge: "file",         cls: "badge-file",         label: "Files" },
    species:      { icon: "\u{1F9AB}", badge: "species",      cls: "badge-species",      label: "Species" },
  };

  // ── Species Data ───────────────────────────────────────────────────

  const SPECIES_DATA = {
    "Human":        { taxid: "9606",  latin: "Homo sapiens",             genome: "GRCh38.p14",  genes: "~20,000", chromosomes: "23 pairs" },
    "Mouse":        { taxid: "10090", latin: "Mus musculus",             genome: "GRCm39",       genes: "~22,000", chromosomes: "20 pairs" },
    "Rat":          { taxid: "10116", latin: "Rattus norvegicus",        genome: "mRatBN7.2",    genes: "~22,000", chromosomes: "21 pairs" },
    "Pig":          { taxid: "9823",  latin: "Sus scrofa",               genome: "Sscrofa11.1",  genes: "~21,000", chromosomes: "19 pairs" },
    "Cattle":       { taxid: "9913",  latin: "Bos taurus",               genome: "ARS-UCD1.2",   genes: "~22,000", chromosomes: "30 pairs" },
    "Dog":          { taxid: "9615",  latin: "Canis lupus familiaris",    genome: "ROS_Cfam_1.0", genes: "~20,000", chromosomes: "39 pairs" },
    "Cat":          { taxid: "9685",  latin: "Felis catus",              genome: "Felis_catus_9.0", genes: "~19,000", chromosomes: "19 pairs" },
    "Rabbit":       { taxid: "9986",  latin: "Oryctolagus cuniculus",    genome: "OryCun2.0",    genes: "~20,000", chromosomes: "22 pairs" },
    "Rhesus macaque":{ taxid: "9544", latin: "Macaca mulatta",           genome: "Mmul_10",      genes: "~21,000", chromosomes: "21 pairs" },
    "Chimpanzee":   { taxid: "9598",  latin: "Pan troglodytes",          genome: "Pan_tro_3.0",  genes: "~20,000", chromosomes: "24 pairs" },
    "Zebrafish":    { taxid: "7955",  latin: "Danio rerio",              genome: "GRCz11",       genes: "~25,000", chromosomes: "25 pairs" },
    "Medaka":       { taxid: "8090",  latin: "Oryzias latipes",          genome: "ASM223467v1",  genes: "~20,000", chromosomes: "24 pairs" },
    "Xenopus":      { taxid: "8364",  latin: "Xenopus tropicalis",       genome: "UCB_Xtro_10.0", genes: "~22,000", chromosomes: "10 pairs" },
    "Fruit fly":    { taxid: "7227",  latin: "Drosophila melanogaster",  genome: "BDGP6",        genes: "~14,000", chromosomes: "4 pairs" },
    "C. elegans":   { taxid: "6239",  latin: "Caenorhabditis elegans",   genome: "WBcel235",     genes: "~20,000", chromosomes: "6" },
    "Honeybee":     { taxid: "7460",  latin: "Apis mellifera",           genome: "Amel_HAv3.1",  genes: "~12,000", chromosomes: "16 pairs" },
    "Arabidopsis":  { taxid: "3702",  latin: "Arabidopsis thaliana",     genome: "TAIR10",       genes: "~27,000", chromosomes: "5 pairs" },
    "Rice":         { taxid: "39947", latin: "Oryza sativa",             genome: "IRGSP-1.0",    genes: "~37,000", chromosomes: "12 pairs" },
    "Maize":        { taxid: "4577",  latin: "Zea mays",                 genome: "Zm-B73-v5.0",  genes: "~40,000", chromosomes: "10 pairs" },
    "Tomato":       { taxid: "4081",  latin: "Solanum lycopersicum",     genome: "SL3.0",        genes: "~35,000", chromosomes: "12 pairs" },
    "Tobacco":      { taxid: "4097",  latin: "Nicotiana tabacum",        genome: "Ntab-TN90",    genes: "~70,000", chromosomes: "24 pairs" },
    "Yeast (S. cerevisiae)": { taxid: "4932",  latin: "Saccharomyces cerevisiae", genome: "R64", genes: "~6,000",  chromosomes: "16" },
    "Yeast (S. pombe)":      { taxid: "4896",  latin: "Schizosaccharomyces pombe", genome: "ASM294v2", genes: "~5,000", chromosomes: "3" },
    "Aspergillus":  { taxid: "5052",  latin: "Aspergillus niger",        genome: "CBS 513.88",   genes: "~11,000", chromosomes: "8" },
    "Candida":      { taxid: "5476",  latin: "Candida albicans",         genome: "SC5314",       genes: "~6,200",  chromosomes: "8" },
    "E. coli":      { taxid: "562",   latin: "Escherichia coli",         genome: "K-12 MG1655",  genes: "~4,300",  chromosomes: "1 circular" },
    "S. aureus":    { taxid: "1280",  latin: "Staphylococcus aureus",    genome: "NCTC 8325",    genes: "~2,800",  chromosomes: "1 circular" },
    "M. tuberculosis":{ taxid: "1773", latin: "Mycobacterium tuberculosis", genome: "H37Rv",     genes: "~4,000",  chromosomes: "1 circular" },
    "P. aeruginosa":{ taxid: "287",   latin: "Pseudomonas aeruginosa",   genome: "PAO1",         genes: "~5,700",  chromosomes: "1 circular" },
    "B. subtilis":  { taxid: "1423",  latin: "Bacillus subtilis",        genome: "168",          genes: "~4,200",  chromosomes: "1 circular" },
    "Salmonella":   { taxid: "28901", latin: "Salmonella enterica",      genome: "LT2",          genes: "~4,500",  chromosomes: "1 circular" },
    "Streptococcus":{ taxid: "1301",  latin: "Streptococcus pneumoniae", genome: "TIGR4",        genes: "~2,200",  chromosomes: "1 circular" },
    "Lactobacillus":{ taxid: "1578",  latin: "Lactobacillus spp.",       genome: "various",      genes: "~2,000-3,000", chromosomes: "1 circular" },
    "H. pylori":    { taxid: "210",   latin: "Helicobacter pylori",      genome: "26695",        genes: "~1,600",  chromosomes: "1 circular" },
    "C. difficile": { taxid: "1496",  latin: "Clostridioides difficile", genome: "630",          genes: "~3,800",  chromosomes: "1 circular" },
    "SARS-CoV-2":   { taxid: "2697049", latin: "Severe acute respiratory syndrome coronavirus 2", genome: "Wuhan-Hu-1", genes: "~12 ORFs", chromosomes: "1 ssRNA" },
    "HIV":          { taxid: "11676", latin: "Human immunodeficiency virus 1", genome: "HXB2",   genes: "9",       chromosomes: "2 ssRNA" },
    "Influenza":    { taxid: "11320", latin: "Influenza A virus",        genome: "various",      genes: "~11",     chromosomes: "8 segments" },
    "Hepatitis B":  { taxid: "10407", latin: "Hepatitis B virus",        genome: "ayw",          genes: "4",       chromosomes: "1 circular dsDNA" },
    "Hepatitis C":  { taxid: "11103", latin: "Hepatitis C virus",        genome: "H77",          genes: "~10",     chromosomes: "1 ssRNA" },
    "HPV":          { taxid: "10566", latin: "Human papillomavirus",     genome: "HPV16",        genes: "8",       chromosomes: "1 circular dsDNA" },
    "EBV":          { taxid: "10376", latin: "Epstein-Barr virus",       genome: "B95-8",        genes: "~85",     chromosomes: "1 linear dsDNA" },
    "HSV":          { taxid: "10298", latin: "Herpes simplex virus",     genome: "17",           genes: "~74",     chromosomes: "1 linear dsDNA" },
  };

  // ── Helpers ────────────────────────────────────────────────────────

  function escapeHtml(s) {
    var d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

  function truncate(s, max) {
    return s.length > max ? s.slice(0, max) + "..." : s;
  }

  function getSnippet(text, pos, radius) {
    radius = radius || 40;
    var start = Math.max(0, pos - radius);
    var end = Math.min(text.length, pos + radius);
    var s = text.substring(start, end).replace(/\s+/g, " ").trim();
    return (start > 0 ? "..." : "") + s + (end < text.length ? "..." : "");
  }

  // ── Detection Functions ────────────────────────────────────────────

  function detectGenes(text) {
    var seen = {};
    var re = /\b([A-Z][A-Z0-9]{1,9})\b/g;
    var match;
    while ((match = re.exec(text)) !== null) {
      var symbol = match[1];
      if (!EXCLUDE.has(symbol) && GENE_SYMBOLS.has(symbol)) {
        if (!seen[symbol]) {
          seen[symbol] = { type: "gene", id: symbol, position: match.index, count: 1, snippet: getSnippet(text, match.index) };
        } else {
          seen[symbol].count++;
        }
      }
    }
    return Object.values(seen);
  }

  function detectVariants(text) {
    var seen = {};
    var match;
    function add(id, subtype, pos) {
      if (!seen[id]) {
        seen[id] = { type: "variant", id: id, subtype: subtype, position: pos, count: 1, snippet: getSnippet(text, pos) };
      } else {
        seen[id].count++;
      }
    }
    var rsRe = /\b(rs\d{3,12})\b/gi;
    while ((match = rsRe.exec(text)) !== null) add(match[1].toLowerCase(), "rsid", match.index);
    var hgvsRe = /\b((?:NM_|NP_|NC_)\d+(?:\.\d+)?:[cpg]\.\S+?)(?=[\s,;)\]]|$)/g;
    while ((match = hgvsRe.exec(text)) !== null) add(match[1], "hgvs", match.index);
    var vcvRe = /\b(VCV\d{9,12})\b/g;
    while ((match = vcvRe.exec(text)) !== null) add(match[1], "clinvar", match.index);
    var cosmicRe = /\b(COSM\d{3,10})\b/g;
    while ((match = cosmicRe.exec(text)) !== null) add(match[1], "cosmic", match.index);
    return Object.values(seen);
  }

  function detectAccessions(text) {
    var results = [];
    var seen = new Set();
    var match;
    var patterns = [
      { re: /\b(GSE\d{3,8})\b/g, subtype: "geo_series" },
      { re: /\b(GSM\d{3,8})\b/g, subtype: "geo_sample" },
      { re: /\b(GPL\d{2,6})\b/g, subtype: "geo_platform" },
      { re: /\b(SRR\d{5,10})\b/g, subtype: "sra_run" },
      { re: /\b(SRP\d{5,10})\b/g, subtype: "sra_project" },
      { re: /\b(SRX\d{5,10})\b/g, subtype: "sra_experiment" },
      { re: /\b(ERR\d{5,10})\b/g, subtype: "ena_run" },
      { re: /\b(ERX\d{5,10})\b/g, subtype: "ena_experiment" },
      { re: /\b(DRR\d{5,10})\b/g, subtype: "ddbj_run" },
      { re: /\b(PRJNA\d{4,8})\b/g, subtype: "bioproject" },
      { re: /\b(PRJEB\d{4,8})\b/g, subtype: "bioproject_eu" },
      { re: /\b(PRJDB\d{4,8})\b/g, subtype: "bioproject_jp" },
      { re: /\b(SAMN\d{6,10})\b/g, subtype: "biosample" },
      { re: /\b(ENSG\d{11})\b/g, subtype: "ensembl_gene" },
      { re: /\b(ENST\d{11})\b/g, subtype: "ensembl_transcript" },
      { re: /\b(NM_\d{6,9}(?:\.\d+)?)\b/g, subtype: "refseq" },
      { re: /\b(GCA_\d{9}\.\d)\b/g, subtype: "genbank_assembly" },
      { re: /\b(GCF_\d{9}\.\d)\b/g, subtype: "refseq_assembly" },
      { re: /\b(10\.\d{4,9}\/[^\s]{5,50})\b/g, subtype: "doi" },
    ];
    for (var i = 0; i < patterns.length; i++) {
      var p = patterns[i];
      while ((match = p.re.exec(text)) !== null) {
        var id = match[1];
        if (p.subtype === "doi") id = id.replace(/[.,;)\]]+$/, "");
        if (!seen.has(id)) {
          seen.add(id);
          results.push({ type: "accession", id: id, subtype: p.subtype, position: match.index, snippet: getSnippet(text, match.index) });
        }
      }
    }
    return results;
  }

  function detectSpecies(text) {
    var results = [];
    var patterns = [
      // Mammals
      { re: /\b(?:Homo sapiens|human)\b/gi, name: "Human" },
      { re: /\b(?:Mus musculus|mouse|murine)\b/gi, name: "Mouse" },
      { re: /\b(?:Rattus norvegicus|rat)\b/gi, name: "Rat" },
      { re: /\b(?:Sus scrofa|pig|porcine|swine)\b/gi, name: "Pig" },
      { re: /\b(?:Bos taurus|cattle|bovine)\b/gi, name: "Cattle" },
      { re: /\b(?:Canis lupus familiaris|canine|dog)\b/gi, name: "Dog" },
      { re: /\b(?:Felis catus|feline|cat)\b/gi, name: "Cat" },
      { re: /\b(?:Oryctolagus cuniculus|rabbit)\b/gi, name: "Rabbit" },
      { re: /\b(?:Macaca mulatta|rhesus macaque)\b/gi, name: "Rhesus macaque" },
      { re: /\b(?:Pan troglodytes|chimpanzee)\b/gi, name: "Chimpanzee" },
      // Fish & amphibians
      { re: /\b(?:Danio rerio|zebrafish)\b/gi, name: "Zebrafish" },
      { re: /\b(?:Oryzias latipes|medaka)\b/gi, name: "Medaka" },
      { re: /\b(?:Xenopus laevis|Xenopus tropicalis|frog|Xenopus)\b/gi, name: "Xenopus" },
      // Invertebrates
      { re: /\b(?:Drosophila melanogaster|fruit fly|Drosophila)\b/gi, name: "Fruit fly" },
      { re: /\b(?:C\. elegans|Caenorhabditis elegans)\b/gi, name: "C. elegans" },
      { re: /\b(?:Apis mellifera|honeybee|honey bee)\b/gi, name: "Honeybee" },
      { re: /\b(?:Anopheles gambiae|mosquito)\b/gi, name: "Mosquito" },
      // Plants
      { re: /\b(?:Arabidopsis thaliana|Arabidopsis)\b/gi, name: "Arabidopsis" },
      { re: /\b(?:Oryza sativa|rice)\b/gi, name: "Rice" },
      { re: /\b(?:Zea mays|maize|corn)\b/gi, name: "Maize" },
      { re: /\b(?:Solanum lycopersicum|tomato)\b/gi, name: "Tomato" },
      { re: /\b(?:Nicotiana tabacum|tobacco)\b/gi, name: "Tobacco" },
      // Fungi
      { re: /\b(?:Saccharomyces cerevisiae|budding yeast|S\. cerevisiae)\b/gi, name: "Yeast (S. cerevisiae)" },
      { re: /\b(?:Schizosaccharomyces pombe|fission yeast|S\. pombe)\b/gi, name: "Yeast (S. pombe)" },
      { re: /\b(?:Aspergillus|A\. niger|A\. fumigatus)\b/gi, name: "Aspergillus" },
      { re: /\b(?:Candida albicans)\b/gi, name: "Candida" },
      // Bacteria
      { re: /\b(?:E\. coli|Escherichia coli)\b/gi, name: "E. coli" },
      { re: /\b(?:Staphylococcus aureus|S\. aureus|MRSA)\b/gi, name: "S. aureus" },
      { re: /\b(?:Mycobacterium tuberculosis|M\. tuberculosis|TB)\b/gi, name: "M. tuberculosis" },
      { re: /\b(?:Pseudomonas aeruginosa|P\. aeruginosa)\b/gi, name: "P. aeruginosa" },
      { re: /\b(?:Bacillus subtilis|B\. subtilis)\b/gi, name: "B. subtilis" },
      { re: /\b(?:Salmonella enterica|Salmonella typhimurium|Salmonella)\b/gi, name: "Salmonella" },
      { re: /\b(?:Streptococcus|S\. pneumoniae|S\. pyogenes)\b/gi, name: "Streptococcus" },
      { re: /\b(?:Lactobacillus|L\. rhamnosus|L\. plantarum)\b/gi, name: "Lactobacillus" },
      { re: /\b(?:Helicobacter pylori|H\. pylori)\b/gi, name: "H. pylori" },
      { re: /\b(?:Clostridium difficile|C\. difficile|C\. diff)\b/gi, name: "C. difficile" },
      // Viruses
      { re: /\b(?:SARS-CoV-2|COVID-19|coronavirus)\b/gi, name: "SARS-CoV-2" },
      { re: /\b(?:HIV|HIV-1|human immunodeficiency virus)\b/gi, name: "HIV" },
      { re: /\b(?:influenza|H1N1|H5N1|H3N2)\b/gi, name: "Influenza" },
      { re: /\b(?:hepatitis B|HBV)\b/gi, name: "Hepatitis B" },
      { re: /\b(?:hepatitis C|HCV)\b/gi, name: "Hepatitis C" },
      { re: /\b(?:HPV|human papillomavirus)\b/gi, name: "HPV" },
      { re: /\b(?:Epstein-Barr|EBV)\b/gi, name: "EBV" },
      { re: /\b(?:herpes simplex|HSV-1|HSV-2)\b/gi, name: "HSV" },
    ];
    var seen = new Set();
    for (var i = 0; i < patterns.length; i++) {
      if (patterns[i].re.test(text) && !seen.has(patterns[i].name)) {
        seen.add(patterns[i].name);
        results.push({ type: "species", id: patterns[i].name });
      }
    }
    return results;
  }

  function detectFileLinks(text) {
    var results = [];
    var seen = new Set();
    var re = /(https?:\/\/\S+\.(?:fasta|fa|fastq|fq|vcf|bed|gff|gtf|bam|sam|csv|tsv)(?:\.gz)?)/gi;
    var match;
    while ((match = re.exec(text)) !== null) {
      if (!seen.has(match[1])) {
        seen.add(match[1]);
        results.push({ type: "file", id: match[1] });
      }
    }
    return results;
  }

  // ── Methods/Tools detection ────────────────────────────────────────

  function detectMethods(text) {
    var tools = [
      // Aligners
      "BWA","BWA-MEM","BWA-MEM2","Bowtie","Bowtie2","STAR","HISAT2","minimap2","GMAP","GSNAP",
      "TopHat","TopHat2","Novoalign","BBMap","Subread","LAST","Bismark","BSMAP",
      // Quantification
      "Salmon","Kallisto","RSEM","Cufflinks","StringTie","featureCounts","HTSeq","Sailfish",
      // Variant calling
      "GATK","HaplotypeCaller","Mutect2","DeepVariant","bcftools","FreeBayes","Strelka","Strelka2",
      "VarScan","VarScan2","Platypus","Delly","Manta","LUMPY","GRIDSS","SvABA",
      "CNVkit","FACETS","Sequenza","ASCAT","BIC-seq2",
      // Alignment/BAM tools
      "samtools","Picard","BEDTools","BEDOPS","sambamba","mosdepth","goleft",
      // QC & preprocessing
      "FastQC","MultiQC","Trimmomatic","fastp","cutadapt","bbduk","Trim Galore","AfterQC",
      "Qualimap","RSeQC","Preseq","DupRadar",
      // Sequence search
      "BLAST","DIAMOND","HMMER","MMseqs2","CD-HIT","USEARCH","VSEARCH",
      // Annotation
      "VEP","ANNOVAR","SnpEff","SnpSift","Funcotator","InterProScan","Pfam","PANTHER",
      "DAVID","Enrichr","GSEA","MSigDB","Reactome",
      // DE & expression
      "DESeq2","edgeR","limma","sleuth","ballgown","NOISeq","baySeq","EBSeq",
      // Single cell
      "Seurat","Scanpy","CellRanger","Monocle","Monocle3","scran","scater",
      "Velocyto","scVelo","SCENIC","CellChat","Harmony","LIGER","Scrublet","DoubletFinder",
      // ChIP-seq / ATAC-seq
      "MACS2","MACS3","Homer","DiffBind","ChIPseeker","SICER","GEM","nf-core/chipseq",
      // Methylation
      "Bismark","methylKit","BSseeker","MethylDackel","DMRfinder",
      // Metagenomics
      "Kraken2","MetaPhlAn","MetaPhlAn4","QIIME","QIIME2","mothur","DADA2","PICRUSt2",
      "HUMAnN","Bracken","Centrifuge","Kaiju","MEGAHIT","SPAdes","metaSPAdes",
      // Assembly
      "SPAdes","Velvet","ABySS","Canu","Flye","Hifiasm","wtdbg2","Shasta","Verkko",
      // Phylogenetics
      "RAxML","IQ-TREE","BEAST","BEAST2","MrBayes","PhyML","FastTree","MEGA",
      "MAFFT","MUSCLE","Clustal Omega","ClustalW","T-Coffee","PRANK",
      // Structure
      "AlphaFold","ColabFold","RoseTTAFold","ESMFold","PyMOL","Chimera","ChimeraX","VMD",
      "GROMACS","AMBER","NAMD","OpenMM","AutoDock","MOE",
      // Workflow
      "Nextflow","Snakemake","WDL","CWL","Cromwell","Galaxy","Toil","Airflow",
      "nf-core","nf-core/rnaseq","nf-core/sarek","nf-core/viralrecon",
      // Containers
      "Docker","Singularity","Apptainer","Conda","Bioconda","BioContainers",
      // Languages & frameworks
      "R/Bioconductor","Python","Bioconductor","BioPython","Biopython","BioPerl","Biojulia",
      "ggplot2","matplotlib","seaborn","plotly","pandas","NumPy","SciPy",
      "tidyverse","dplyr","data.table",
      // GWAS
      "PLINK","PLINK2","GCTA","LDSC","BOLT-LMM","SAIGE","REGENIE","METAL","LocusZoom",
      // Databases
      "IGV","UCSC Genome Browser","Ensembl VEP",
      "ClinVar","gnomAD","dbSNP","COSMIC","PharmGKB","OMIM","CIViC",
      "UniProt","PDB","STRING","KEGG","Gene Ontology",
      // Imputation
      "IMPUTE2","IMPUTE5","Minimac4","Beagle","SHAPEIT","Eagle",
      // Long read
      "Dorado","Guppy","Medaka","Nanopolish","NanoPlot","NanoStat",
      "pbmm2","IsoSeq","Lima","CCS"
    ];
    var results = [];
    var seen = new Set();
    for (var i = 0; i < tools.length; i++) {
      var t = tools[i];
      var re = new RegExp("\\b" + t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\b", t === t.toUpperCase() ? "g" : "gi");
      if (re.test(text) && !seen.has(t.toLowerCase())) {
        seen.add(t.toLowerCase());
        // Try to find version
        var verRe = new RegExp("\\b" + t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "[\\s/v]*(\\d+\\.\\d+[\\w.]*)", "i");
        var ver = verRe.exec(text);
        results.push({ type: "method", id: t, version: ver ? ver[1] : null });
      }
    }
    return results;
  }

  // ── Genome build detection ────────────────────────────────────────

  function detectGenomeBuild(text) {
    var results = [];
    var builds = [
      { re: /\b(GRCh38|hg38)\b/gi, id: "GRCh38 (hg38)" },
      { re: /\b(GRCh37|hg19)\b/gi, id: "GRCh37 (hg19)" },
      { re: /\b(T2T-CHM13|T2T)\b/gi, id: "T2T-CHM13" },
      { re: /\b(GRCm39|mm39)\b/gi, id: "GRCm39 (mm39)" },
      { re: /\b(GRCm38|mm10)\b/gi, id: "GRCm38 (mm10)" },
      { re: /\b(hg18)\b/gi, id: "hg18 (NCBI36)" },
    ];
    var seen = new Set();
    for (var i = 0; i < builds.length; i++) {
      if (builds[i].re.test(text) && !seen.has(builds[i].id)) {
        seen.add(builds[i].id);
        results.push({ type: "genome_build", id: builds[i].id });
      }
    }
    return results;
  }

  // ── Sample size detection ─────────────────────────────────────────

  function detectSampleSize(text) {
    var results = [];
    var patterns = [
      /\b[Nn]\s*=\s*(\d{1,6})\b/g,
      /\b(\d{1,6})\s+(?:samples|patients|subjects|individuals|participants|cases|controls)\b/gi,
      /\bcohort\s+of\s+(\d{1,6})\b/gi,
    ];
    var seen = new Set();
    for (var i = 0; i < patterns.length; i++) {
      var match;
      while ((match = patterns[i].exec(text)) !== null) {
        var n = match[1];
        if (parseInt(n) >= 3 && !seen.has(n)) {
          seen.add(n);
          results.push({ type: "sample_size", id: "n=" + n, snippet: getSnippet(text, match.index) });
        }
      }
    }
    return results;
  }

  // ── Statistical methods detection ─────────────────────────────────

  function detectStatMethods(text) {
    var methods = [
      "t-test","Student's t","Mann-Whitney","Wilcoxon","chi-square","chi-squared",
      "Fisher's exact","ANOVA","Kruskal-Wallis","log-rank","Cox regression",
      "logistic regression","linear regression","Kaplan-Meier",
      "Bonferroni","Benjamini-Hochberg","FDR","false discovery rate",
      "Bayesian","MCMC","bootstrap","permutation test",
      "PCA","principal component","t-SNE","UMAP",
      "k-means","hierarchical clustering","random forest",
      "support vector","neural network","deep learning",
      "p-value","p < 0.05","p < 0.01","fold change","log2FC",
      "odds ratio","hazard ratio","confidence interval",
      "Pearson","Spearman","Kendall"
    ];
    var results = [];
    var seen = new Set();
    for (var i = 0; i < methods.length; i++) {
      var m = methods[i];
      var re = new RegExp("\\b" + m.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\b", "gi");
      if (re.test(text) && !seen.has(m.toLowerCase())) {
        seen.add(m.toLowerCase());
        results.push({ type: "stat_method", id: m });
      }
    }
    return results;
  }

  // ── Sequencing platform detection ─────────────────────────────────

  function detectPlatforms(text) {
    var platforms = [
      { re: /\b(Illumina|HiSeq|NovaSeq|NovaSeq X|MiSeq|NextSeq|iSeq)\b/gi, id: "Illumina" },
      { re: /\b(PacBio|SMRT|Sequel|Sequel II|Revio|HiFi)\b/gi, id: "PacBio" },
      { re: /\b(Oxford Nanopore|MinION|PromethION|GridION|Nanopore|ONT)\b/gi, id: "Oxford Nanopore" },
      { re: /\b(10x Genomics|10X|Chromium|Visium|Xenium)\b/gi, id: "10x Genomics" },
      { re: /\b(Ion Torrent|Ion Proton|Ion S5|Ion GeneStudio)\b/gi, id: "Ion Torrent" },
      { re: /\b(MGI|MGISEQ|DNBSEQ)\b/g, id: "MGI/DNBSEQ" },
      { re: /\b(Element Biosciences|AVITI)\b/gi, id: "Element AVITI" },
      { re: /\b(Ultima Genomics|UG100)\b/gi, id: "Ultima Genomics" },
      { re: /\b(SOLiD)\b/g, id: "SOLiD" },
      { re: /\b(Sanger sequencing)\b/gi, id: "Sanger" },
      { re: /\b(Spatial Transcriptomics|Slide-seq|MERFISH|seqFISH)\b/gi, id: "Spatial Transcriptomics" },
    ];
    var results = [];
    var seen = new Set();
    for (var i = 0; i < platforms.length; i++) {
      if (platforms[i].re.test(text) && !seen.has(platforms[i].id)) {
        seen.add(platforms[i].id);
        results.push({ type: "platform", id: platforms[i].id });
      }
    }
    return results;
  }

  // ── Cell line detection ───────────────────────────────────────────

  function detectCellLines(text) {
    var lines = [
      // Cervical / epithelial
      "HeLa","SiHa","CaSki","C-33A","ME-180","MS751","SW756",
      // Kidney
      "HEK293","HEK293T","HEK293FT","MDCK","Vero","786-O","ACHN","Caki-1","Caki-2",
      "A498","RPTEC","RCC4","OS-RC-2","769-P","UOK101",
      // Breast
      "MCF7","MCF-7","MCF10A","MDA-MB-231","MDA-MB-468","T47D","ZR-75-1","SK-BR-3",
      "BT-474","BT-549","SUM149","SUM159","SUM185","HCC1806","HCC1937","HCC1954",
      "HCC38","MDA-MB-453","CAL-51","MDA-MB-436","Hs578T","AU565","UACC-812",
      "EFM-19","KPL-1","CAMA-1","MPE600",
      // Lung
      "A549","H460","H1299","H1975","H358","H322","PC-9","HCC827","NCI-H460",
      "NCI-H1650","NCI-H2228","NCI-H3122","H520","H226","H69","H82",
      "NCI-H292","Calu-1","Calu-3","Calu-6","SK-LU-1","SK-MES-1",
      "SW900","EKVX","HOP-62","HOP-92","NCI-H23","NCI-H322M","NCI-H522",
      "LLC","Lewis lung","EBC-1","H2009","H1048","H446","DMS-79","DMS-114",
      // Colon / colorectal
      "HCT116","SW480","SW620","HT-29","LoVo","RKO","DLD-1","Caco-2","HCT-8",
      "HT-116","COLO205","COLO320","KM12","HCT-15","SW48","SW837","LS174T",
      "T84","Ls180","NCI-H716","GP5d","WiDr","CT26","MC38",
      // Liver / hepatocellular
      "HepG2","Hep3B","Huh7","SNU-449","SK-HEP-1","PLC/PRF/5","Huh6",
      "HLE","HLF","JHH-1","JHH-4","JHH-5","JHH-6","JHH-7","SNU-387","SNU-398",
      "SNU-182","SNU-423","SNU-475","MHCC97H","MHCC97L","SMMC-7721","BEL-7402",
      "LO2","THLE-2","LX-2","HepaRG",
      // Blood / leukemia / lymphoma
      "K562","Jurkat","THP-1","U937","HL-60","MOLT-4","Raji","Ramos","Daudi","NALM-6",
      "KG-1","KG-1a","MV4-11","MOLM-13","OCI-AML3","Kasumi-1","NB4",
      "CCRF-CEM","MOLT-3","SUP-B15","RS4;11","SEM","REH","697","KOPN-8",
      "OCI-LY1","OCI-LY3","OCI-LY7","OCI-LY10","OCI-LY19","SU-DHL-4","SU-DHL-6",
      "SU-DHL-8","SU-DHL-10","DOHH-2","WSU-DLCL2","Toledo","Pfeiffer","DB",
      "JeKo-1","Mino","Z-138","Granta-519","REC-1","MAVER-1",
      "JVM-2","JVM-3","MEC-1","MEC-2","HG-3","WAC-3CD5+",
      "MM.1S","MM.1R","RPMI-8226","U266","NCI-H929","OPM-2","LP-1","KMS-11","KMS-12",
      "L-363","AMO-1","ANBL-6","EJM","JJN-3",
      "EL4","P388","L1210","A20","WEHI-231",
      // Prostate
      "PC-3","LNCaP","DU145","22Rv1","VCaP","C4-2","LAPC4",
      "C4-2B","RWPE-1","RWPE-2","CWR-R1","MDA-PCa-2b","NCI-H660",
      "TRAMP-C2","RM-1","MyC-CaP",
      // Melanoma
      "A375","SK-MEL-28","WM266-4","B16","B16-F10","SK-MEL-2","SK-MEL-5",
      "UACC-257","UACC-62","MDA-MB-435","LOX-IMVI","M14","MALME-3M",
      "WM115","WM793","WM35","451Lu","1205Lu","A2058","A101D",
      "Mewo","IGR-37","IGR-39","SK-MEL-1","SK-MEL-3","SK-MEL-24","SK-MEL-31",
      // Neuronal / glioma / neuroblastoma
      "SH-SY5Y","SK-N-SH","U87","U87MG","U251","T98G","LN229","IMR-32",
      "U118","A172","SF-268","SF-295","SF-539","SNB-19","SNB-75",
      "SK-N-BE(2)","SK-N-AS","SK-N-DZ","SK-N-FI","IMR-5","CHP-212","LAN-1","LAN-5",
      "KELLY","BE(2)-C","BE(2)-M17","NGP","NB-1643","NB-EBc1",
      "PC12","Neuro-2a","N2a","C6","GL261","CT-2A",
      // Bone / sarcoma
      "U2OS","Saos-2","MG-63","SJSA-1","143B","HOS","KHOS",
      "A673","SK-ES-1","RD-ES","TC-71","TC-32","EW8",
      "RD","RH30","RH41","A204","SW982","SW872","HT-1080","MNNG-HOS",
      // Pancreas
      "PANC-1","MiaPaCa-2","BxPC-3","AsPC-1","Capan-1","Capan-2",
      "CFPAC-1","SW1990","HPAF-II","HPAC","Hs766T","SU.86.86","PaTu 8988t",
      "KP-4","Panc 02","Panc 03.27",
      // Ovary
      "SKOV3","OVCAR-3","A2780","ES-2","CAOV3","IGROV1","OVCAR-4","OVCAR-5",
      "OVCAR-8","TOV-21G","TOV-112D","OV-90","COV318","COV362","COV504",
      "PEO1","PEO4","Kuramochi","OVSAHO","JHOS-2","JHOS-4","TYK-nu",
      // Stomach / gastric
      "AGS","MKN45","SNU-1","SNU-16","MKN1","MKN7","MKN28","MKN74",
      "KATO-III","NCI-N87","SNU-5","SNU-484","SNU-601","SNU-638","SNU-668","SNU-719",
      "NUGC-3","NUGC-4","TMK-1","HSC-39",
      // Head & neck
      "FaDu","SCC-9","SCC-15","SCC-25","CAL-27","Detroit 562","UMSCC-1",
      "HN5","SAS","HSC-2","HSC-3","HSC-4",
      // Bladder
      "T24","RT4","5637","J82","TCCSUP","UMUC-3","HT-1376","SW780","RT112",
      // Thyroid
      "TPC-1","BCPAP","K1","FTC-133","8505C","SW1736","CAL-62","HTh7","C643",
      // Endometrial / uterine
      "Ishikawa","ECC-1","HEC-1-A","HEC-1-B","RL95-2","AN3CA","KLE",
      // Esophageal
      "TE-1","TE-5","TE-7","KYSE-30","KYSE-70","KYSE-150","KYSE-410","KYSE-510",
      "OE19","OE21","OE33","FLO-1","EAC","SKGT-4",
      // Mesothelioma
      "MSTO-211H","NCI-H2052","NCI-H226","NCI-H28","NCI-H2452",
      // Normal / immortalized
      "BEAS-2B","HBEC","HMEC","HUVEC","HAEC","hTERT-RPE1","RPE-1","hFOB",
      "WI-38","MRC-5","IMR-90","TIG-1","BJ","HFF","HaCaT","ARPE-19",
      "NHA","NHDF","NHEK","HMC-1","NHLF","16HBE","THLE-3",
      // Other / utility
      "CHO","CHO-K1","BHK-21","NIH3T3","3T3-L1","COS-7","COS-1","HEp-2",
      "L929","RAW264.7","RAW 264.7","J774","P815","RBL-2H3","RBL",
      "WEHI-3","WEHI-164","MEF","3T3","L-cells","SP2/0","NS0",
      "Ba/F3","32D","FDC-P1","FDCP-Mix","M1","D10","CTLL-2",
      // Mouse models
      "4T1","EMT6","E0771","EO771","Py8119","AT-3","PyMT","MMTV",
      "B16-F0","B16-OVA","YUMM1.7","YUMMER1.7","SM1","Cloudman S91",
      "LLC1","KPC","Panc02","ID8","MOC1","MOC2","SCC7","MEER","mEER",
      "Hepa1-6","Pan02","MCA205","MCA38","SA1N","A20","EL4","RMA",
      // Stem cells / iPSC
      "iPSC","ESC","hESC","hiPSC","H1","H9","WA09","WA01",
      "HUES6","HUES8","HUES9","BGO1","BGO2","BG01V","SA02",
      "IMR90-iPSC","WTC-11","PGP1","KOLF2.1J"
    ];
    var results = [];
    var seen = new Set();
    for (var i = 0; i < lines.length; i++) {
      var re = new RegExp("\\b" + lines[i].replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\b", "g");
      if (re.test(text) && !seen.has(lines[i])) {
        seen.add(lines[i]);
        results.push({ type: "cell_line", id: lines[i] });
      }
    }
    return results;
  }

  // ── Tissue/organ detection ────────────────────────────────────────

  function detectTissues(text) {
    var tissues = [
      "blood","serum","plasma","PBMC","bone marrow",
      "brain","cerebral cortex","hippocampus","cerebellum",
      "liver","kidney","lung","heart","pancreas","spleen","thymus",
      "breast","ovary","prostate","colon","stomach","esophagus",
      "skin","muscle","adipose","cartilage","bone",
      "lymph node","tonsil","thyroid","adrenal",
      "tumor","tumour","normal tissue","adjacent normal",
      "biopsy","resection","autopsy",
      "organoid","xenograft","PDX"
    ];
    var results = [];
    var seen = new Set();
    for (var i = 0; i < tissues.length; i++) {
      var re = new RegExp("\\b" + tissues[i].replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\b", "gi");
      if (re.test(text) && !seen.has(tissues[i].toLowerCase())) {
        seen.add(tissues[i].toLowerCase());
        results.push({ type: "tissue", id: tissues[i] });
      }
    }
    return results;
  }

  // ── Drug name detection ───────────────────────────────────────────

  function detectDrugs(text) {
    var drugs = [
      // PARP inhibitors
      "olaparib","talazoparib","rucaparib","niraparib","veliparib",
      // Immune checkpoint inhibitors
      "pembrolizumab","nivolumab","atezolizumab","durvalumab","ipilimumab","avelumab","cemiplimab",
      "tremelimumab","dostarlimab","retifanlimab","toripalimab","sintilimab","camrelizumab",
      // Monoclonal antibodies — targeted
      "trastuzumab","cetuximab","bevacizumab","rituximab","obinutuzumab","pertuzumab",
      "ramucirumab","panitumumab","daratumumab","elotuzumab","mogamulizumab",
      "brentuximab vedotin","trastuzumab deruxtecan","sacituzumab govitecan",
      "enfortumab vedotin","polatuzumab vedotin","belantamab mafodotin",
      "ado-trastuzumab emtansine","gemtuzumab ozogamicin",
      // BCR-ABL / tyrosine kinase
      "imatinib","dasatinib","nilotinib","ponatinib","bosutinib","asciminib",
      // EGFR inhibitors
      "erlotinib","gefitinib","osimertinib","afatinib","lapatinib","neratinib","tucatinib",
      "dacomitinib","mobocertinib","amivantamab",
      // BRAF/MEK inhibitors
      "vemurafenib","dabrafenib","trametinib","cobimetinib","binimetinib","encorafenib",
      // ALK/ROS1 inhibitors
      "crizotinib","alectinib","ceritinib","lorlatinib","brigatinib","entrectinib",
      // CDK4/6 inhibitors
      "palbociclib","ribociclib","abemaciclib","trilaciclib",
      // BCL-2 / BTK inhibitors
      "venetoclax","ibrutinib","acalabrutinib","zanubrutinib","pirtobrutinib",
      // IMiDs
      "lenalidomide","pomalidomide","thalidomide","avadomide","iberdomide",
      // Cytotoxic chemotherapy
      "doxorubicin","cisplatin","carboplatin","oxaliplatin","paclitaxel","docetaxel",
      "fluorouracil","5-FU","gemcitabine","capecitabine","methotrexate","pemetrexed",
      "irinotecan","topotecan","etoposide","vincristine","vinblastine","vinorelbine",
      "cyclophosphamide","ifosfamide","busulfan","melphalan","chlorambucil","bendamustine",
      "bleomycin","mitomycin","dactinomycin","epirubicin","daunorubicin","mitoxantrone",
      "cytarabine","azacitidine","decitabine","fludarabine","cladribine","clofarabine",
      "hydroxyurea","mercaptopurine","thioguanine","tretinoin","arsenic trioxide",
      // Hormonal therapy
      "tamoxifen","letrozole","anastrozole","exemestane","fulvestrant",
      "enzalutamide","abiraterone","apalutamide","darolutamide","bicalutamide","flutamide",
      "leuprolide","goserelin","degarelix","relugolix",
      // Multi-kinase / VEGFR
      "sorafenib","sunitinib","pazopanib","cabozantinib","axitinib","lenvatinib",
      "regorafenib","vandetanib","tivozanib","nintedanib",
      // FGFR inhibitors
      "erdafitinib","pemigatinib","futibatinib","infigratinib",
      // FLT3 inhibitors
      "midostaurin","gilteritinib","quizartinib",
      // IDH inhibitors
      "ivosidenib","enasidenib","vorasidenib",
      // mTOR / PI3K inhibitors
      "everolimus","rapamycin","sirolimus","temsirolimus",
      "alpelisib","idelalisib","copanlisib","duvelisib","umbralisib",
      // Hedgehog inhibitors
      "vismodegib","sonidegib","glasdegib",
      // Proteasome inhibitors
      "bortezomib","carfilzomib","ixazomib",
      // HDAC inhibitors
      "vorinostat","romidepsin","panobinostat","belinostat","tucidinostat",
      // RAS/KRAS
      "sotorasib","adagrasib",
      // HER2
      "margetuximab","zanidatamab",
      // Other targeted
      "temozolomide","bexarotene","alitretinoin","panobinostat",
      "selinexor","belzutifan","tazemetostat","tivantinib",
      "larotrectinib","selpercatinib","pralsetinib","capmatinib","tepotinib",
      "avapritinib","ripretinib","elacestrant",
      // CAR-T & bispecifics
      "tisagenlecleucel","axicabtagene ciloleucel","brexucabtagene autoleucel",
      "lisocabtagene maraleucel","idecabtagene vicleucel","ciltacabtagene autoleucel",
      "blinatumomab","mosunetuzumab","glofitamab","epcoritamab","teclistamab","talquetamab",
      // Steroids
      "dexamethasone","prednisone","prednisolone","methylprednisolone","hydrocortisone",
      // Cardiovascular
      "aspirin","metformin","atorvastatin","simvastatin","rosuvastatin","pravastatin",
      "warfarin","heparin","enoxaparin","rivaroxaban","apixaban","dabigatran","clopidogrel",
      "lisinopril","enalapril","ramipril","losartan","valsartan","amlodipine","nifedipine",
      "metoprolol","atenolol","propranolol","carvedilol","digoxin","amiodarone",
      // Diabetes
      "insulin","glipizide","glyburide","pioglitazone","rosiglitazone",
      "sitagliptin","saxagliptin","linagliptin","empagliflozin","dapagliflozin","canagliflozin",
      "liraglutide","semaglutide","dulaglutide","exenatide","tirzepatide",
      // Anti-inflammatory / autoimmune
      "adalimumab","infliximab","etanercept","golimumab","certolizumab",
      "tocilizumab","sarilumab","baricitinib","tofacitinib","upadacitinib","ruxolitinib",
      "abatacept","secukinumab","ixekizumab","ustekinumab","guselkumab","risankizumab",
      "dupilumab","omalizumab","mepolizumab","benralizumab",
      "hydroxychloroquine","sulfasalazine","mycophenolate","azathioprine","tacrolimus","cyclosporine",
      "colchicine","ibuprofen","naproxen","celecoxib","indomethacin",
      // Antibiotics
      "amoxicillin","ampicillin","penicillin","cephalexin","ceftriaxone","cefepime",
      "azithromycin","erythromycin","clarithromycin","doxycycline","tetracycline","minocycline",
      "ciprofloxacin","levofloxacin","moxifloxacin","vancomycin","linezolid","daptomycin",
      "metronidazole","trimethoprim","sulfamethoxazole","nitrofurantoin","rifampin","isoniazid",
      "gentamicin","tobramycin","amikacin","colistin","polymyxin",
      "meropenem","imipenem","piperacillin","clindamycin",
      // Antivirals
      "acyclovir","valacyclovir","ganciclovir","oseltamivir","remdesivir","molnupiravir",
      "nirmatrelvir","ritonavir","tenofovir","emtricitabine","lamivudine","zidovudine",
      "dolutegravir","raltegravir","efavirenz","rilpivirine","sofosbuvir","ledipasvir",
      // Antifungals
      "fluconazole","itraconazole","voriconazole","posaconazole","amphotericin B",
      "caspofungin","micafungin","anidulafungin","nystatin","terbinafine",
      // CNS / psychiatry
      "sertraline","fluoxetine","paroxetine","citalopram","escitalopram","venlafaxine",
      "duloxetine","bupropion","mirtazapine","trazodone","amitriptyline","nortriptyline",
      "lithium","valproic acid","carbamazepine","lamotrigine","levetiracetam","phenytoin",
      "gabapentin","pregabalin","topiramate","clonazepam","lorazepam","diazepam",
      "haloperidol","olanzapine","quetiapine","risperidone","aripiprazole","clozapine",
      "donepezil","memantine","rivastigmine","levodopa","carbidopa","pramipexole","ropinirole",
      // Analgesics
      "morphine","fentanyl","oxycodone","hydromorphone","codeine","tramadol",
      "acetaminophen","ketorolac","lidocaine","bupivacaine",
      // Respiratory
      "albuterol","ipratropium","fluticasone","budesonide","montelukast",
      "tiotropium","formoterol","salmeterol",
      // GI
      "omeprazole","pantoprazole","lansoprazole","esomeprazole","famotidine","ranitidine",
      "ondansetron","metoclopramide","loperamide",
      // Research compounds
      "DMSO","rapamycin","staurosporine","thapsigargin","tunicamycin","bafilomycin",
      "nocodazole","colchicine","actinomycin D","puromycin","cycloheximide","chloroquine",
      "wortmannin","U0126","PD98059","LY294002","SB203580","SP600125","BMS-345541",
      "MG-132","bortezomib","nutlin-3","JQ1","I-BET762","GSK126","EPZ-6438","tazemetostat"
    ];
    var results = [];
    var seen = new Set();
    for (var i = 0; i < drugs.length; i++) {
      var re = new RegExp("\\b" + drugs[i].replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\b", "gi");
      if (re.test(text) && !seen.has(drugs[i].toLowerCase())) {
        seen.add(drugs[i].toLowerCase());
        results.push({ type: "drug", id: drugs[i] });
      }
    }
    return results;
  }

  // ── Clinical trial ID detection ──────────────────────────────────

  function detectClinicalTrials(text) {
    var results = [];
    var seen = new Set();
    var re = /\b(NCT\d{8,11})\b/g;
    var m;
    while ((m = re.exec(text)) !== null) {
      if (!seen.has(m[1])) {
        seen.add(m[1]);
        results.push({ type: "clinical_trial", id: m[1], snippet: getSnippet(text, m.index) });
      }
    }
    return results;
  }

  // ── Funding / grant ID detection ────────────────────────────────

  function detectFunding(text) {
    var results = [];
    var seen = new Set();
    var patterns = [
      // NIH grants: R01/R21/U01/P01/K08/T32/F31 etc.
      { re: /\b([RPUKFTMS]\d{2}\s?[A-Z]{2}\d{5,8})\b/g, source: "NIH" },
      // NSF grants
      { re: /\bNSF[\s#-]*([\d]{7})\b/g, source: "NSF" },
      // ERC grants
      { re: /\b(ERC-\d{4}-(?:StG|CoG|AdG|SyG|PoC))\b/gi, source: "ERC" },
      // Wellcome Trust
      { re: /\bWellcome[\s#]*(\d{6,9})\b/gi, source: "Wellcome" },
      // HHMI
      { re: /\b(HHMI)\b/g, source: "HHMI" },
      // Generic grant number patterns (conservative)
      { re: /\bgrant[\s#:]*(?:no\.?\s*)?([A-Z0-9]{2,4}[\s/-]?[A-Z0-9]{4,12})\b/gi, source: "Grant" },
    ];
    for (var i = 0; i < patterns.length; i++) {
      var p = patterns[i];
      p.re.lastIndex = 0;
      var m;
      while ((m = p.re.exec(text)) !== null) {
        var id = (p.source === "NSF" ? "NSF " : "") + m[1].trim();
        if (p.source === "HHMI") id = "HHMI";
        if (!seen.has(id) && id.length >= 4) {
          seen.add(id);
          results.push({ type: "funding", id: id, subtype: p.source, snippet: getSnippet(text, m.index) });
        }
      }
    }
    return results;
  }

  // ── Repository link detection ───────────────────────────────────

  function detectRepositories(text) {
    var results = [];
    var seen = new Set();
    var patterns = [
      // GitHub
      { re: /\bhttps?:\/\/github\.com\/[\w.-]+\/[\w.-]+\b/g, source: "GitHub" },
      // Zenodo
      { re: /\bhttps?:\/\/zenodo\.org\/(?:record|doi)\/[\w.-]+\b/g, source: "Zenodo" },
      { re: /\b(10\.5281\/zenodo\.\d+)\b/g, source: "Zenodo DOI" },
      // Figshare
      { re: /\bhttps?:\/\/figshare\.com\/articles\/[\w.-/]+\b/g, source: "Figshare" },
      // Dryad
      { re: /\bhttps?:\/\/datadryad\.org\/stash\/dataset\/[\w.-]+\b/g, source: "Dryad" },
      // Bitbucket
      { re: /\bhttps?:\/\/bitbucket\.org\/[\w.-]+\/[\w.-]+\b/g, source: "Bitbucket" },
      // GitLab
      { re: /\bhttps?:\/\/gitlab\.com\/[\w.-]+\/[\w.-]+\b/g, source: "GitLab" },
      // CRAN / Bioconductor packages
      { re: /\bhttps?:\/\/(?:cran\.r-project\.org|bioconductor\.org)\/packages\/[\w.-]+\b/g, source: "R Package" },
      // PyPI
      { re: /\bhttps?:\/\/pypi\.org\/project\/[\w.-]+\b/g, source: "PyPI" },
    ];
    for (var i = 0; i < patterns.length; i++) {
      var p = patterns[i];
      p.re.lastIndex = 0;
      var m;
      while ((m = p.re.exec(text)) !== null) {
        var id = m[1] || m[0];
        if (!seen.has(id)) {
          seen.add(id);
          results.push({ type: "repository", id: id, subtype: p.source });
        }
      }
    }
    return results;
  }

  // ── P-value detection ───────────────────────────────────────────

  function detectPValues(text) {
    var results = [];
    var seen = new Set();
    var patterns = [
      // p < 0.05, p = 0.001, P < 1e-8, p-value = 3.2×10⁻⁸
      /[Pp][\s-]*(?:value)?[\s]*[<>=≤≥]\s*([\d.]+\s*[×x×]\s*10\s*[⁻-]\s*\d+)/g,
      /[Pp][\s-]*(?:value)?[\s]*[<>=≤≥]\s*([\d.eE-]+)/g,
      // FDR < 0.05, q < 0.01, adjusted p
      /(?:FDR|q[\s-]*value|adjusted\s+[Pp])[\s]*[<>=≤≥]\s*([\d.eE-]+)/g,
    ];
    for (var i = 0; i < patterns.length; i++) {
      patterns[i].lastIndex = 0;
      var m;
      while ((m = patterns[i].exec(text)) !== null) {
        var full = m[0].trim();
        if (!seen.has(full) && full.length >= 5) {
          seen.add(full);
          results.push({ type: "p_value", id: full, snippet: getSnippet(text, m.index) });
        }
      }
    }
    // Deduplicate overlapping matches
    var unique = [];
    var uids = new Set();
    results.forEach(function(r) { if (!uids.has(r.id)) { uids.add(r.id); unique.push(r); } });
    return unique.slice(0, 20); // cap to avoid noise
  }

  // ── Key findings extraction ───────────────────────────────────────

  function extractKeyFindings(text, genes) {
    if (!genes || genes.length === 0) return [];
    var sentences = text.split(/[.!?]\s+/);
    var keywords = /\b(significant|associated|correlated|mutation|pathogenic|overexpress|downregulat|upregulat|loss.of.function|gain.of.function|driver|resistance|sensitiv|prognos|biomarker|therapeutic|target|inhibit|activat|essential|critical|novel|recurrent)\b/i;
    var results = [];
    var geneSet = new Set(genes.map(function(g) { return typeof g === "string" ? g : g.id; }));
    for (var i = 0; i < sentences.length && results.length < 10; i++) {
      var s = sentences[i].trim();
      if (s.length < 30 || s.length > 500) continue;
      var hasGene = false;
      geneSet.forEach(function(g) { if (s.includes(g)) hasGene = true; });
      if (hasGene && keywords.test(s)) {
        results.push({ type: "finding", id: s.substring(0, 150) + (s.length > 150 ? "..." : "") });
      }
    }
    return results;
  }

  // ── Full scan ──────────────────────────────────────────────────────

  function scanText(text) {
    var entities = [];
    var genes = detectGenes(text);
    entities = entities.concat(genes);
    entities = entities.concat(detectVariants(text));
    entities = entities.concat(detectAccessions(text));
    entities = entities.concat(detectSpecies(text));
    entities = entities.concat(detectFileLinks(text));
    entities = entities.concat(detectMethods(text));
    entities = entities.concat(detectGenomeBuild(text));
    entities = entities.concat(detectSampleSize(text));
    entities = entities.concat(detectStatMethods(text));
    entities = entities.concat(detectPlatforms(text));
    entities = entities.concat(detectCellLines(text));
    entities = entities.concat(detectTissues(text));
    entities = entities.concat(detectDrugs(text));
    entities = entities.concat(detectClinicalTrials(text));
    entities = entities.concat(detectFunding(text));
    entities = entities.concat(detectRepositories(text));
    entities = entities.concat(detectPValues(text));
    entities = entities.concat(extractKeyFindings(text, genes));
    return entities;
  }

  // ── Source Links Builder ───────────────────────────────────────────

  function buildSourceLinks(type, name, info) {
    var links = [];
    var geneId = info && info["Gene ID"];
    var taxid = info && info["Taxonomy ID"];

    if (type === "gene") {
      if (geneId) links.push({ label: "NCBI Gene", url: "https://www.ncbi.nlm.nih.gov/gene/" + geneId });
      links.push({ label: "Ensembl", url: "https://ensembl.org/Homo_sapiens/Gene/Summary?g=" + name });
      if (info && info["UniProt"]) links.push({ label: "UniProt", url: "https://www.uniprot.org/uniprot/" + info["UniProt"] });
      links.push({ label: "GeneCards", url: "https://www.genecards.org/cgi-bin/carddisp.pl?gene=" + name });
    } else if (type === "variant") {
      if (/^rs\d+$/i.test(name)) {
        links.push({ label: "dbSNP", url: "https://www.ncbi.nlm.nih.gov/snp/" + name });
        links.push({ label: "gnomAD", url: "https://gnomad.broadinstitute.org/variant/" + name });
        links.push({ label: "ClinVar", url: "https://www.ncbi.nlm.nih.gov/clinvar/?term=" + name });
      }
    } else if (type === "accession") {
      if (/^GSE\d+$/i.test(name)) links.push({ label: "GEO", url: "https://www.ncbi.nlm.nih.gov/geo/query/acc.cgi?acc=" + name });
      if (/^SRR\d+$/i.test(name)) links.push({ label: "SRA", url: "https://www.ncbi.nlm.nih.gov/sra/" + name });
      if (/^PRJNA\d+$/i.test(name)) links.push({ label: "BioProject", url: "https://www.ncbi.nlm.nih.gov/bioproject/" + name });
      if (/^ENSG\d+$/i.test(name)) links.push({ label: "Ensembl", url: "https://ensembl.org/id/" + name });
      if (/^10\.\d{4}/.test(name)) links.push({ label: "DOI", url: "https://doi.org/" + name });
    } else if (type === "species") {
      if (taxid) links.push({ label: "NCBI Taxonomy", url: "https://www.ncbi.nlm.nih.gov/Taxonomy/Browser/wwwtax.cgi?id=" + taxid });
    }
    return links;
  }

  // ── Export Builders ────────────────────────────────────────────────

  function groupEntities(entities) {
    var grouped = { gene: [], variant: [], accession: [], file: [], species: [] };
    if (Array.isArray(entities)) {
      entities.forEach(function(e) {
        var t = e.type || "accession";
        if (!grouped[t]) grouped[t] = [];
        grouped[t].push(e);
      });
    }
    return grouped;
  }

  function buildMarkdown(grouped, sources) {
    var md = "## BioGist Scan Results\n\n";
    if (sources && sources.length > 0) {
      md += "**Sources (" + sources.length + "):**\n";
      sources.forEach(function(s) { md += "- " + s.title + " (" + s.count + " entities)\n"; });
      md += "\n";
    }
    md += "*Exported: " + new Date().toISOString() + "*\n\n";
    for (var type in grouped) {
      var items = grouped[type];
      if (!items || items.length === 0) continue;
      var meta = TYPE_META[type];
      md += "### " + (meta ? meta.label : type) + "\n\n";
      items.forEach(function(e) {
        var id = typeof e === "string" ? e : e.id;
        md += "**" + id + "**";
        if (e.source) md += " *(from: " + e.source + ")*";
        if (e.count && e.count > 1) md += " (×" + e.count + ")";
        md += "\n";
        if (e.snippet) md += "> " + e.snippet + "\n";
        md += "\n";
      });
    }
    return md;
  }

  function buildJson(grouped, sources) {
    var obj = {};
    for (var type in grouped) {
      if (grouped[type] && grouped[type].length > 0) {
        obj[type] = grouped[type].map(function(e) {
          return typeof e === "string" ? { id: e } : e;
        });
      }
    }
    if (sources) obj._sources = sources;
    obj._exported = new Date().toISOString();
    return JSON.stringify(obj, null, 2);
  }

  // ── Public API ─────────────────────────────────────────────────────

  window.BioGistCore = {
    GENE_SYMBOLS: GENE_SYMBOLS,
    EXCLUDE: EXCLUDE,
    TYPE_META: TYPE_META,
    SPECIES_DATA: SPECIES_DATA,
    escapeHtml: escapeHtml,
    truncate: truncate,
    getSnippet: getSnippet,
    detectGenes: detectGenes,
    detectVariants: detectVariants,
    detectAccessions: detectAccessions,
    detectSpecies: detectSpecies,
    detectFileLinks: detectFileLinks,
    detectMethods: detectMethods,
    detectGenomeBuild: detectGenomeBuild,
    detectSampleSize: detectSampleSize,
    detectStatMethods: detectStatMethods,
    detectPlatforms: detectPlatforms,
    detectCellLines: detectCellLines,
    detectTissues: detectTissues,
    detectDrugs: detectDrugs,
    detectClinicalTrials: detectClinicalTrials,
    detectFunding: detectFunding,
    detectRepositories: detectRepositories,
    detectPValues: detectPValues,
    scanText: scanText,
    buildSourceLinks: buildSourceLinks,
    groupEntities: groupEntities,
    buildMarkdown: buildMarkdown,
    buildJson: buildJson,
  };

})();
