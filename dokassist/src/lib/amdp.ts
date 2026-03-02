// AMDP (Association for Methodology and Documentation in Psychiatry)
// Psychopathological findings structure

export interface AMDPItem {
  code: string;
  label: string;
  score: 0 | 1 | 2 | 3; // not present | mild | moderate | severe
}

export interface AMDPCategory {
  name: string;
  items: AMDPItem[];
}

// Complete AMDP structure with 12 categories
export const AMDP_CATEGORIES: AMDPCategory[] = [
  {
    name: 'Bewusstsein',
    items: [
      { code: 'Bew-1', label: 'Bewusstseinsverminderung', score: 0 },
      { code: 'Bew-2', label: 'Bewusstseinsverschiebung', score: 0 },
      { code: 'Bew-3', label: 'Bewusstseinseinengung', score: 0 },
      { code: 'Bew-4', label: 'Ratlosigkeit', score: 0 },
    ],
  },
  {
    name: 'Aufmerksamkeit und Gedächtnis',
    items: [
      { code: 'Auf-1', label: 'Auffassungsstörungen', score: 0 },
      { code: 'Auf-2', label: 'Konzentrationsstörungen', score: 0 },
      { code: 'Auf-3', label: 'Merkfähigkeitsstörungen', score: 0 },
      { code: 'Auf-4', label: 'Erinnerungsstörungen', score: 0 },
      { code: 'Auf-5', label: 'Konfabulation', score: 0 },
      { code: 'Auf-6', label: 'Gedächtnisstörungen', score: 0 },
    ],
  },
  {
    name: 'Formales Denken',
    items: [
      { code: 'FoD-1', label: 'Gedankendrängen', score: 0 },
      { code: 'FoD-2', label: 'Gehemmt-verlangsamtes Denken', score: 0 },
      { code: 'FoD-3', label: 'Grübeln', score: 0 },
      { code: 'FoD-4', label: 'Perseveration', score: 0 },
      { code: 'FoD-5', label: 'Gedankenabreißen', score: 0 },
      { code: 'FoD-6', label: 'Ideenflüchtigkeit', score: 0 },
      { code: 'FoD-7', label: 'Vorbeireden', score: 0 },
      { code: 'FoD-8', label: 'Zerfahrenheit', score: 0 },
      { code: 'FoD-9', label: 'Inkohärenz', score: 0 },
      { code: 'FoD-10', label: 'Neologismen', score: 0 },
    ],
  },
  {
    name: 'Befürchtungen und Zwänge',
    items: [
      { code: 'Bef-1', label: 'Misstrauen', score: 0 },
      { code: 'Bef-2', label: 'Hypochondrie', score: 0 },
      { code: 'Bef-3', label: 'Phobien', score: 0 },
      { code: 'Bef-4', label: 'Zwangsdenken', score: 0 },
      { code: 'Bef-5', label: 'Zwangsimpulse', score: 0 },
      { code: 'Bef-6', label: 'Zwangshandlungen', score: 0 },
    ],
  },
  {
    name: 'Wahn',
    items: [
      { code: 'Wah-1', label: 'Wahnstimmung', score: 0 },
      { code: 'Wah-2', label: 'Wahnwahrnehmung', score: 0 },
      { code: 'Wah-3', label: 'Wahneinfall', score: 0 },
      { code: 'Wah-4', label: 'Wahngedanken', score: 0 },
      { code: 'Wah-5', label: 'Wahnsystem', score: 0 },
      { code: 'Wah-6', label: 'Verarmungswahn', score: 0 },
      { code: 'Wah-7', label: 'Schuldwahn', score: 0 },
      { code: 'Wah-8', label: 'Hypochondrischer Wahn', score: 0 },
      { code: 'Wah-9', label: 'Nihilistischer Wahn', score: 0 },
      { code: 'Wah-10', label: 'Beziehungswahn', score: 0 },
      { code: 'Wah-11', label: 'Beeinträchtigungswahn', score: 0 },
      { code: 'Wah-12', label: 'Verfolgungswahn', score: 0 },
      { code: 'Wah-13', label: 'Eifersuchtswahn', score: 0 },
      { code: 'Wah-14', label: 'Liebeswahn', score: 0 },
      { code: 'Wah-15', label: 'Größenwahn', score: 0 },
    ],
  },
  {
    name: 'Sinnestäuschungen',
    items: [
      { code: 'Sin-1', label: 'Illusionen', score: 0 },
      { code: 'Sin-2', label: 'Halluzinationen akustisch', score: 0 },
      { code: 'Sin-3', label: 'Halluzinationen optisch', score: 0 },
      { code: 'Sin-4', label: 'Halluzinationen olfaktorisch', score: 0 },
      { code: 'Sin-5', label: 'Halluzinationen gustatorisch', score: 0 },
      { code: 'Sin-6', label: 'Halluzinationen körperhaft', score: 0 },
    ],
  },
  {
    name: 'Ich-Störungen',
    items: [
      { code: 'Ich-1', label: 'Depersonalisation', score: 0 },
      { code: 'Ich-2', label: 'Derealisation', score: 0 },
      { code: 'Ich-3', label: 'Gedankenausbreitung', score: 0 },
      { code: 'Ich-4', label: 'Gedankenentzug', score: 0 },
      { code: 'Ich-5', label: 'Gedankeneingebung', score: 0 },
      { code: 'Ich-6', label: 'Fremdbeeinflussungserleben', score: 0 },
    ],
  },
  {
    name: 'Orientierung',
    items: [
      { code: 'Ori-1', label: 'Zeitlich desorientiert', score: 0 },
      { code: 'Ori-2', label: 'Örtlich desorientiert', score: 0 },
      { code: 'Ori-3', label: 'Situativ desorientiert', score: 0 },
      { code: 'Ori-4', label: 'Persönlich desorientiert', score: 0 },
    ],
  },
  {
    name: 'Affektivität',
    items: [
      { code: 'Aff-1', label: 'Ratlosigkeit', score: 0 },
      { code: 'Aff-2', label: 'Insuffizienzgefühle', score: 0 },
      { code: 'Aff-3', label: 'Schuldgefühle', score: 0 },
      { code: 'Aff-4', label: 'Verarmungsgefühle', score: 0 },
      { code: 'Aff-5', label: 'Resignative Einstellung', score: 0 },
      { code: 'Aff-6', label: 'Hoffnungslosigkeit', score: 0 },
      { code: 'Aff-7', label: 'Selbstmordgedanken', score: 0 },
      { code: 'Aff-8', label: 'Ängstlichkeit', score: 0 },
      { code: 'Aff-9', label: 'Dysphorische Stimmung', score: 0 },
      { code: 'Aff-10', label: 'Depressive Stimmung', score: 0 },
      { code: 'Aff-11', label: 'Gefühl der Gefühllosigkeit', score: 0 },
      { code: 'Aff-12', label: 'Euphorie', score: 0 },
      { code: 'Aff-13', label: 'Ekstatische Entrückung', score: 0 },
      { code: 'Aff-14', label: 'Affektlabilität', score: 0 },
      { code: 'Aff-15', label: 'Affektinkontinenz', score: 0 },
      { code: 'Aff-16', label: 'Parathymie', score: 0 },
      { code: 'Aff-17', label: 'Affektverflachung', score: 0 },
    ],
  },
  {
    name: 'Antrieb und Psychomotorik',
    items: [
      { code: 'Ant-1', label: 'Antriebsminderung', score: 0 },
      { code: 'Ant-2', label: 'Antriebshemmung', score: 0 },
      { code: 'Ant-3', label: 'Antriebssteigerung', score: 0 },
      { code: 'Ant-4', label: 'Motorische Unruhe', score: 0 },
      { code: 'Ant-5', label: 'Bewegungsstereotypien', score: 0 },
      { code: 'Ant-6', label: 'Manierismen', score: 0 },
      { code: 'Ant-7', label: 'Paramimie', score: 0 },
      { code: 'Ant-8', label: 'Grimassieren', score: 0 },
      { code: 'Ant-9', label: 'Katalepsie', score: 0 },
      { code: 'Ant-10', label: 'Negativismus', score: 0 },
      { code: 'Ant-11', label: 'Mutismus', score: 0 },
    ],
  },
  {
    name: 'Schlaf',
    items: [
      { code: 'Sch-1', label: 'Einschlafstörungen', score: 0 },
      { code: 'Sch-2', label: 'Durchschlafstörungen', score: 0 },
      { code: 'Sch-3', label: 'Früherwachen', score: 0 },
      { code: 'Sch-4', label: 'Schlafvertiefung', score: 0 },
    ],
  },
  {
    name: 'Sonstiges',
    items: [
      { code: 'Son-1', label: 'Appetitverminderung', score: 0 },
      { code: 'Son-2', label: 'Appetitsteigerung', score: 0 },
      { code: 'Son-3', label: 'Gewichtsverlust', score: 0 },
      { code: 'Son-4', label: 'Gewichtszunahme', score: 0 },
      { code: 'Son-5', label: 'Libidoverminderung', score: 0 },
      { code: 'Son-6', label: 'Libidosteigerung', score: 0 },
      { code: 'Son-7', label: 'Insuffizienzgefühle', score: 0 },
      { code: 'Son-8', label: 'Fehlende Krankheitseinsicht', score: 0 },
    ],
  },
];

// Helper function to serialize AMDP data to JSON
export function serializeAMDP(categories: AMDPCategory[]): string {
  return JSON.stringify(categories);
}

// Helper function to deserialize AMDP data from JSON
export function deserializeAMDP(json: string | null): AMDPCategory[] {
  if (!json) {
    return JSON.parse(JSON.stringify(AMDP_CATEGORIES)); // Deep clone
  }
  try {
    return JSON.parse(json);
  } catch {
    return JSON.parse(JSON.stringify(AMDP_CATEGORIES));
  }
}
