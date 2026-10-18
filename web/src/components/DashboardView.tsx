import React, { useState, useEffect } from 'react';
import { UserProfile, JobData } from '../types';
import { Wallet, Server, ArrowRight, Clock, Plus, ExternalLink } from 'lucide-react';

interface DashboardViewProps {
  user: UserProfile;
  onTopupClick: () => void;
  onSelectJob: (id: number) => void;
  onNewInstallClick: () => void;
}

export const DashboardView: React.FC<DashboardViewProps> = ({
  user,
  onTopupClick,
  onSelectJob,
  onNewInstallClick,
}) => {
  const [jobs, setJobs] = useState<JobData[]>([]);
  const [loading, setLoading] = useState<boolean>(true);

  useEffect(() => {
    fetch('/dashboard', { headers: { Accept: 'application/json' } })
      .then((r) => r.json())
      .then((data) => {
        if (Array.isArray(data.jobs)) setJobs(data.jobs);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="max-w-4xl w-full mx-auto space-y-5 sm:space-y-6">
      {/* Saldo & Quick Action Card */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3.5 sm:gap-4">
        <div className="card-bone p-4 sm:p-6 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between text-[#71717a] text-xs font-bold uppercase tracking-wider mb-1.5 sm:mb-2">
              <span>Saldo Akun</span>
              <Wallet className="w-4 h-4 text-orange-500" />
            </div>
            <div className="text-2xl sm:text-3xl font-extrabold text-[#0f172a] tracking-tight">
              Rp{user.balance.toLocaleString('id-ID')}
            </div>
            <p className="text-xs text-[#71717a] mt-1">Dapat digunakan untuk instalasi berakun.</p>
          </div>
          <div className="mt-4 sm:mt-5">
            <button
              onClick={onTopupClick}
              className="w-full sm:w-auto px-4 py-2 rounded-lg border border-[#e8e4dc] text-xs font-bold text-[#0f172a] hover:bg-[#f4f1ea] flex items-center justify-center gap-1.5"
            >
              <Plus className="w-4 h-4 text-orange-500" />
              <span>Isi Saldo (QRIS)</span>
            </button>
          </div>
        </div>

        <div className="card-bone p-4 sm:p-6 flex flex-col justify-between bg-gradient-to-br from-white to-orange-50/40">
          <div>
            <div className="flex items-center justify-between text-[#71717a] text-xs font-bold uppercase tracking-wider mb-1.5 sm:mb-2">
              <span>Instalasi Berakun</span>
              <Server className="w-4 h-4 text-orange-500" />
            </div>
            <div className="text-2xl sm:text-3xl font-extrabold text-[#0f172a] tracking-tight">
              Rp5.000<span className="text-xs font-normal text-[#71717a] ml-1">/ instalasi</span>
            </div>
            <p className="text-xs text-[#71717a] mt-1">
              Pasang Windows langsung pada VPS Linux Anda tanpa kode voucer.
            </p>
          </div>
          <div className="mt-4 sm:mt-5">
            <button
              onClick={onNewInstallClick}
              className="btn-orange w-full sm:w-auto px-5 py-2 rounded-lg text-xs font-bold flex items-center justify-center gap-1.5"
            >
              <span>Pasang Sekarang</span>
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {/* Riwayat Pekerjaan */}
      <div className="card-bone p-4 sm:p-7">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-bold text-[#0f172a] flex items-center gap-2">
            <Clock className="w-4 h-4 text-orange-500" />
            <span>Riwayat Instalasi Anda</span>
          </h2>
          <span className="text-xs text-[#71717a]">10 terakhir</span>
        </div>

        {loading ? (
          <div className="p-8 text-center text-xs text-[#71717a]">Memuat riwayat...</div>
        ) : jobs.length === 0 ? (
          <div className="text-center py-10 border border-dashed border-[#e8e4dc] rounded-xl text-xs text-[#71717a]">
            Belum ada instalasi berakun yang dijalankan.
          </div>
        ) : (
          <div className="divide-y divide-[#e8e4dc]">
            {jobs.map((j) => (
              <div
                key={j.id}
                onClick={() => onSelectJob(j.id)}
                className="py-3.5 flex items-center justify-between gap-4 hover:bg-[#fbfaf7] px-2 rounded-lg cursor-pointer transition-colors"
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div className="w-8 h-8 rounded-lg bg-[#f4f1ea] font-mono text-xs font-bold text-[#0f172a] flex items-center justify-center flex-shrink-0">
                    #{j.id}
                  </div>
                  <div className="min-w-0">
                    <p className="text-xs sm:text-sm font-bold text-[#0f172a] truncate">{j.os_label}</p>
                    <p className="text-[11px] font-mono text-[#71717a] truncate">{j.host}</p>
                  </div>
                </div>

                <div className="flex items-center gap-3 flex-shrink-0">
                  <span
                    className={`px-2.5 py-0.5 rounded-full text-[11px] font-bold uppercase tracking-wider ${
                      j.status === 'done'
                        ? 'bg-emerald-100 text-emerald-800'
                        : j.status === 'failed'
                        ? 'bg-red-100 text-red-800'
                        : 'bg-orange-100 text-orange-800'
                    }`}
                  >
                    {j.status}
                  </span>
                  <ExternalLink className="w-4 h-4 text-[#a1a1aa]" />
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
