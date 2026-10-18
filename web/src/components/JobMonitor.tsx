import React, { useState, useEffect } from 'react';
import { JobData } from '../types';
import {
  CheckCircle2,
  Clock,
  Server,
  Terminal,
  Copy,
  Check,
  AlertTriangle,
  RotateCcw,
  Trash2,
  Monitor,
  Key,
  Shield,
  Layers,
  Sparkles,
} from 'lucide-react';

interface JobMonitorProps {
  token?: string;
  jobId?: number;
  initialData?: JobData;
  onBack?: () => void;
}

export const JobMonitor: React.FC<JobMonitorProps> = ({
  token,
  jobId,
  initialData,
  onBack,
}) => {
  const [job, setJob] = useState<JobData | null>(initialData || null);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [error, setError] = useState<string>('');

  const fetchUrl = token ? `/g/${token}` : `/j/${jobId}`;
  const sseUrl = token ? `/g/${token}/events` : `/j/${jobId}/events`;

  useEffect(() => {
    // Initial fetch if no data
    const load = async () => {
      try {
        const res = await fetch(fetchUrl, {
          headers: { Accept: 'application/json' },
        });
        if (res.ok) {
          const data = await res.json();
          setJob(data);
        }
      } catch (err: any) {
        // SSE will catch up or polling
      }
    };

    if (!job) load();

    // Setup SSE
    let es: EventSource | null = null;
    try {
      es = new EventSource(sseUrl);
      es.addEventListener('card', (ev) => {
        try {
          const parsed = JSON.parse(ev.data);
          setJob(parsed);
        } catch {
          // If HTML returned, refresh state
          load();
        }
      });
      es.onerror = () => {
        es?.close();
      };
    } catch {
      // Ignore
    }

    // Polling fallback every 6 seconds
    const interval = setInterval(load, 6000);

    return () => {
      clearInterval(interval);
      es?.close();
    };
  }, [fetchUrl, sseUrl]);

  const copyToClipboard = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const getRailStep = () => {
    if (!job) return 0;
    if (job.status === 'done') return 4;
    if (job.status === 'failed') return -1;
    const p = (job.phase || '').toLowerCase();
    if (p.includes('rdp') || p.includes('stabil')) return 3;
    if (p.includes('reboot')) return 2;
    if (p.includes('unduh') || p.includes('mengirim')) return 1;
    return 0;
  };

  const currentStep = getRailStep();

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      {/* Top Banner */}
      <div className="card-bone p-6 sm:p-7">
        <div className="flex items-center justify-between gap-4 mb-4 pb-4 border-b border-[#e8e4dc]">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-orange-100 flex items-center justify-center text-orange-600 font-bold text-xs">
              #{job?.id || '...'}
            </div>
            <div>
              <h1 className="text-base sm:text-lg font-bold text-[#0f172a]">
                {job?.os_label || 'Pemasangan Windows'}
              </h1>
              <p className="text-xs text-[#71717a] font-mono">{job?.host || 'Menghubungi VPS'}</p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <span
              className={`px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wider ${
                job?.status === 'done'
                  ? 'bg-emerald-100 text-emerald-800 border border-emerald-200'
                  : job?.status === 'failed'
                  ? 'bg-red-100 text-red-800 border border-red-200'
                  : 'bg-orange-100 text-orange-800 border border-orange-200 animate-pulse'
              }`}
            >
              {job?.status || 'menunggu'}
            </span>
          </div>
        </div>

        {/* 4-Step Rail */}
        <div className="mb-6">
          <div className="grid grid-cols-4 gap-2">
            {[
              { idx: 0, label: '1. Periksa VPS' },
              { idx: 1, label: '2. Unduh OS' },
              { idx: 2, label: '3. Reboot' },
              { idx: 3, label: '4. RDP Siap' },
            ].map(({ idx, label }) => {
              const isPast = currentStep > idx;
              const isActive = currentStep === idx;
              const isBad = currentStep === -1 && idx === 0;

              return (
                <div key={idx} className="flex flex-col gap-2">
                  <div
                    className={`h-1.5 rounded-full transition-all duration-500 ${
                      isPast
                        ? 'bg-emerald-500'
                        : isActive
                        ? 'bg-orange-500 shadow-sm shadow-orange-500/50 animate-pulse'
                        : isBad
                        ? 'bg-red-500'
                        : 'bg-[#e2ded4]'
                    }`}
                  />
                  <span
                    className={`text-[11px] font-semibold text-center truncate ${
                      isActive ? 'text-orange-600 font-bold' : isPast ? 'text-emerald-700' : 'text-[#71717a]'
                    }`}
                  >
                    {label}
                  </span>
                </div>
              );
            })}
          </div>
        </div>

        {/* Current Phase Message */}
        <div className="p-4 rounded-xl bg-[#f4f1ea] border border-[#e2ded4] flex items-center gap-3">
          {job?.status === 'done' ? (
            <CheckCircle2 className="w-5 h-5 text-emerald-600 flex-shrink-0" />
          ) : job?.status === 'failed' ? (
            <AlertTriangle className="w-5 h-5 text-red-600 flex-shrink-0" />
          ) : (
            <div className="w-4 h-4 rounded-full border-2 border-orange-500 border-t-transparent animate-spin flex-shrink-0" />
          )}
          <span className="text-xs sm:text-sm font-semibold text-[#0f172a]">
            {job?.phase || 'Menyiapkan koneksi ke target...'}
          </span>
        </div>

        {/* Error message */}
        {job?.error && (
          <div className="mt-4 p-4 rounded-xl bg-red-50 border border-red-200 text-red-700 text-xs sm:text-sm flex items-start gap-2.5">
            <AlertTriangle className="w-4 h-4 text-red-500 flex-shrink-0 mt-0.5" />
            <div>
              <b>Pemasangan terhenti:</b> {job.error}
            </div>
          </div>
        )}

        {/* Credentials Card (When DONE) */}
        {job?.status === 'done' && (
          <div className="mt-6 p-5 rounded-2xl bg-gradient-to-br from-emerald-50 to-teal-50 border border-emerald-200 space-y-4 shadow-sm">
            <div className="flex items-center gap-2 text-emerald-900 font-bold text-sm">
              <CheckCircle2 className="w-5 h-5 text-emerald-600" />
              <span>Windows Berhasil Dipasang — Kredensial RDP Anda</span>
            </div>

            <div className="space-y-2.5">
              {[
                { label: 'Alamat Host / IP', value: job.host, id: 'host' },
                { label: 'Port RDP', value: String(job.rdp_port), id: 'port' },
                { label: 'Username', value: job.rdp_user, id: 'user' },
                { label: 'Password RDP', value: job.password || '(Sudah Dihapus dari Cache)', id: 'pw' },
              ].map(({ label, value, id }) => (
                <div
                  key={id}
                  className="p-3 bg-white/90 rounded-xl border border-emerald-200/80 flex items-center justify-between gap-3 text-xs sm:text-sm"
                >
                  <span className="text-[#71717a] font-medium">{label}</span>
                  <div className="flex items-center gap-2">
                    <span className="font-mono font-bold text-[#0f172a]">{value}</span>
                    <button
                      type="button"
                      onClick={() => copyToClipboard(value, id)}
                      className="p-1.5 rounded-lg hover:bg-emerald-100 text-emerald-700 transition-colors"
                      title="Salin"
                    >
                      {copiedKey === id ? <Check className="w-4 h-4 text-emerald-600" /> : <Copy className="w-4 h-4" />}
                    </button>
                  </div>
                </div>
              ))}
            </div>

            <p className="text-xs text-emerald-800 leading-relaxed">
              Buka aplikasi <b>Remote Desktop Connection</b> (Windows) atau <b>Microsoft Remote Desktop</b> (Mac/Android/iOS), lalu masukkan host dan kredensial di atas.
            </p>
          </div>
        )}

        {/* Facts Sheet */}
        <div className="mt-6 pt-5 border-t border-[#e8e4dc] grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
          <div>
            <span className="text-[#71717a] block">Target Host</span>
            <span className="font-mono font-bold text-[#0f172a]">{job?.host}:{job?.ssh_port}</span>
          </div>
          <div>
            <span className="text-[#71717a] block">Versi Windows</span>
            <span className="font-bold text-[#0f172a]">{job?.os_label}</span>
          </div>
          <div>
            <span className="text-[#71717a] block">Waktu Mulai</span>
            <span className="text-[#0f172a]">{job ? new Date(job.created_at * 1000).toLocaleTimeString('id-ID') : '-'}</span>
          </div>
          <div>
            <span className="text-[#71717a] block">Selesai</span>
            <span className="text-[#0f172a]">
              {job?.done_at ? new Date(job.done_at * 1000).toLocaleTimeString('id-ID') : 'Sedang berjalan'}
            </span>
          </div>
        </div>
      </div>

      {/* Activity Log */}
      {job?.events && job.events.length > 0 && (
        <div className="card-bone p-5">
          <h2 className="text-xs font-bold uppercase tracking-wider text-[#71717a] mb-3 flex items-center gap-2">
            <Terminal className="w-4 h-4 text-orange-500" />
            <span>Riwayat Aktivitas Mesin</span>
          </h2>

          <div className="space-y-2 max-h-60 overflow-y-auto pr-1">
            {job.events.map((ev, i) => (
              <div key={i} className="text-xs flex items-start gap-2.5 p-2 rounded-lg bg-[#f4f1ea]/60">
                <span className="font-mono text-[#a1a1aa] flex-shrink-0 text-[11px]">
                  {new Date(ev.ts * 1000).toLocaleTimeString('id-ID')}
                </span>
                <div className="flex-1">
                  <span className="font-bold text-[#0f172a]">{ev.phase}</span>
                  {ev.message && <p className="text-[#71717a] text-[11px] mt-0.5">{ev.message}</p>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Action Buttons */}
      <div className="flex items-center justify-between gap-4">
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            className="px-4 py-2 rounded-lg border border-[#e8e4dc] text-xs font-bold text-[#52525b] hover:bg-white"
          >
            ← Kembali ke Halaman Utama
          </button>
        )}
      </div>
    </div>
  );
};
