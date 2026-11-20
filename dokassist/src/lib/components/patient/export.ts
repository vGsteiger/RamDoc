/**
 * Patient exports, moved unchanged from the old overview's Export dropdown so
 * the header's ⋯ menu can offer them on every patient route.
 */
import { save } from '@tauri-apps/plugin-dialog';
import { exportFhirBundle, exportPatientPdf, type Patient } from '$lib/api';

const today = () => new Date().toISOString().split('T')[0];

export async function exportPatientFhir(patient: Patient): Promise<void> {
  const fhirJson = await exportFhirBundle(patient.id);
  const fileName = `FHIR_${patient.last_name}_${patient.first_name}_${today()}.json`;
  const filePath = await save({
    defaultPath: fileName,
    filters: [{ name: 'FHIR Bundle', extensions: ['json'] }],
  });
  if (filePath) {
    const { writeTextFile } = await import('@tauri-apps/plugin-fs');
    await writeTextFile(filePath, fhirJson);
  }
}

export async function exportPatientSummaryPdf(patient: Patient): Promise<void> {
  const bytes = await exportPatientPdf(patient.id);
  const blob = new Blob([new Uint8Array(bytes)], { type: 'application/pdf' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `Patient_${patient.last_name}_${patient.first_name}_${today()}.pdf`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
