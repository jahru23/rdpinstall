import React, { useState } from 'react';
import { EDITIONS } from '../types';
import {
  Ticket,
  Server,
  Eye,
  EyeOff,
  CheckCircle2,
  AlertCircle,
  ArrowRight,
  ArrowLeft,
  Terminal,
  Shield,
  Layers,
  Sparkles,
  HelpCircle,
} from 'lucide-react';

interface GuestWizardProps {
  onSuccess: (token: string) => void;
}

export const GuestWizard: React.FC<GuestWizardProps> = ({ onSuccess }) => {
  const [step, setStep] = useState<number>(1);
  const [error, setError] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);

  // Form State
  const [voucher, setVoucher] = useState<string>('');
  const [host, setHost] = useState<string>('');
  const [sshPort, setSshPort] = useState<string>('22');
  const [sshUser, setSshUser] = useState<string>('root');
  const [sshPass, setSshPass] = useState<string>('');
  const [showSshPass, setShowSshPass] = useState<boolean>(false);

  const [edition, setEdition] = useState<string>('srv2022');
  const [lang, setLang] = useState<string>('en-us');
  const [rdpUser, setRdpUser] = useState<string>('Administrator');
  const [rdpPass, setRdpPass] = useState<string>('');
  const [showRdpPass, setShowRdpPass] = useState<boolean>(false);
  const [rdpPort, setRdpPort] = useState<string>('3389');

  const selectedEditionObj = EDITIONS.find((e) => e.key === edition) || EDITIONS[0];

  const validateStep = (currentStep: number): boolean => {
    setError('');
    if (currentStep === 1) {
      if (!voucher.trim()) {
        setError('Silakan masukkan kode voucer.');
        return false;
      }
      return true;
    }
    if (currentStep === 2) {
      if (!host.trim()) {
        setError('Alamat IP publik atau domain VPS wajib diisi.');
        return false;
      }
      if (!sshPass) {
        setError('Password SSH root wajib diisi.');
        return false;
      }
      return true;
    }
    if (currentStep === 3) {
      if (!rdpPass) {
        setError('Password Windows wajib diisi.');
        return false;
      }
      if (rdpPass.length < 8) {
        setError('Password Windows minimal 8 karakter.');
        return false;
      }
      return true;
    }
    return true;
  };

  const handleNext = () => {
    if (validateStep(step)) {
      setStep((prev) => Math.min(4, prev + 1));
    }
  };

  const handleBack = () => {
    setError('');
    setStep((prev) => Math.max(1, prev - 1));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validateStep(3)) return;

    setLoading(true);
    setError('');

    try {
      const formBody = new URLSearchParams();
      formBody.append('voucher', voucher);
      formBody.append('host', host);
      formBody.append('ssh_port', sshPort);
      formBody.append('ssh_user', sshUser);
      formBody.append('ssh_pass', sshPass);
      formBody.append('edition', edition);
      formBody.append('lang', lang);
      formBody.append('rdp_user', rdpUser);
      formBody.append('rdp_pass', rdpPass);
      formBody.append('rdp_port', rdpPort);

      const res = await fetch('/', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: formBody.toString(),
      });

      if (res.redirected) {
        const url = new URL(res.url);
        const token = url.pathname.replace('/g/', '');
        onSuccess(token);
        return;
      }

      if (!res.ok) {
        const text = await res.text();
        const match = text.match(/class="alert">([^<]+)/);
        setError(match ? match[1].trim() : 'Gagal memulai instalasi. Periksa kembali data Anda.');
      } else {
        const text = await res.text();
        const match = text.match(/class="alert">([^<]+)/);
        if (match) {
          setError(match[1].trim());
        }
      }
    } catch (err: any) {
      setError(err?.message || 'Terjadi kesalahan jaringan.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="max-w-2xl w-full mx-auto">
      {/* Intro Header */}
      <div className="text-center mb-6 sm:mb-8 px-1">
        <div className="inline-flex items-center gap-1.5 sm:gap-2 px-3 py-1 sm:py-1.5 rounded-full bg-orange-50 border border-orange-200/80 text-orange-700 text-[11px] sm:text-xs font-semibold mb-2.5 sm:mb-3 shadow-xs max-w-full">
          <Sparkles className="w-3.5 h-3.5 text-orange-500 flex-shrink-0" />
          <span className="truncate">Universal Linux to Windows RDP</span>
        </div>
        <h1 className="text-xl sm:text-3xl font-extrabold text-[#0f172a] tracking-tight">
          VPS Linux Anda, dijadikan Windows.
        </h1>
        <p className="mt-2 text-xs sm:text-base text-[#52525b] max-w-lg mx-auto">
          Cukup masukkan kode voucer dan data akses SSH VPS Anda. Sistem akan memeriksa mesin dan menginstal Windows otomatis.
        </p>
      </div>

      {/* Stepper Header */}
      <div className="card-bone p-2.5 sm:p-5 mb-5 sm:mb-6">
        <div className="grid grid-cols-4 gap-1 sm:gap-2 text-center">
          {[
            { s: 1, title: 'Voucer', short: 'Voucer' },
            { s: 2, title: 'Akses VPS', short: 'VPS' },
            { s: 3, title: 'OS Windows', short: 'Windows' },
            { s: 4, title: 'Konfirmasi', short: 'Review' },
          ].map(({ s, title, short }) => (
            <button
              key={s}
              type="button"
              disabled={s > step}
              onClick={() => s < step && setStep(s)}
              className={`flex flex-col items-center gap-1 sm:gap-1.5 p-1.5 sm:p-2 rounded-lg transition-all min-w-0 ${
                step === s
                  ? 'bg-[#0f172a] text-white shadow-sm'
                  : s < step
                  ? 'text-orange-600 hover:bg-orange-50'
                  : 'text-[#a1a1aa] cursor-not-allowed'
              }`}
            >
              <div
                className={`w-5 h-5 sm:w-6 sm:h-6 rounded-full flex items-center justify-center text-[10px] sm:text-xs font-bold flex-shrink-0 ${
                  step === s
                    ? 'bg-orange-500 text-white'
                    : s < step
                    ? 'bg-orange-100 text-orange-700'
                    : 'bg-[#f4f1ea] text-[#71717a]'
                }`}
              >
                {s < step ? <CheckCircle2 className="w-3.5 h-3.5 sm:w-4 sm:h-4" /> : s}
              </div>
              <span className="text-[10px] sm:text-xs font-medium truncate max-w-full">
                <span className="sm:hidden">{short}</span>
                <span className="hidden sm:inline">{title}</span>
              </span>
            </button>
          ))}
        </div>
      </div>

      {/* Error Alert */}
      {error && (
        <div className="mb-5 sm:mb-6 p-3 sm:p-4 rounded-xl bg-red-50 border border-red-200 text-red-700 text-xs sm:text-sm flex items-start gap-2.5 sm:gap-3 shadow-xs">
          <AlertCircle className="w-4 h-4 sm:w-5 sm:h-5 flex-shrink-0 text-red-500 mt-0.5" />
          <div className="flex-1 break-words">{error}</div>
        </div>
      )}

      {/* Card Form */}
      <div className="card-bone p-4 sm:p-8">
        {/* STEP 1: VOUCHER */}
        {step === 1 && (
          <div className="space-y-4 sm:space-y-5">
            <div>
              <h2 className="text-base sm:text-lg font-bold text-[#0f172a] flex items-center gap-2">
                <Ticket className="w-4 h-4 sm:w-5 sm:h-5 text-orange-500 flex-shrink-0" />
                <span>Masukkan Kode Voucer</span>
              </h2>
              <p className="text-xs sm:text-sm text-[#52525b] mt-1">
                Voucer sekali pakai untuk satu kali sesi instalasi Windows.
              </p>
            </div>

            <div>
              <label htmlFor="voucher" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-2">
                Kode Voucer
              </label>
              <div className="relative">
                <input
                  id="voucher"
                  name="voucher"
                  type="text"
                  required
                  placeholder="RDP-XXXX-XXXX"
                  value={voucher}
                  onChange={(e) => setVoucher(e.target.value.toUpperCase())}
                  className="input-bone w-full px-3.5 sm:px-4 py-2.5 sm:py-3 text-base sm:text-lg font-mono tracking-wider uppercase font-bold placeholder:font-normal placeholder:tracking-normal"
                  autoFocus
                />
              </div>
              <p className="text-[11px] sm:text-xs text-[#71717a] mt-1.5">
                Dapatkan kode voucer dari admin atau bot Telegram jaystore.
              </p>
            </div>

            <div className="pt-3 sm:pt-4 flex justify-end">
              <button
                type="button"
                onClick={handleNext}
                className="btn-orange w-full sm:w-auto px-6 py-2.5 rounded-lg font-bold text-sm flex items-center justify-center gap-2"
              >
                <span>Lanjutkan</span>
                <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}

        {/* STEP 2: VPS ACCESS */}
        {step === 2 && (
          <div className="space-y-4 sm:space-y-5">
            <div>
              <h2 className="text-base sm:text-lg font-bold text-[#0f172a] flex items-center gap-2">
                <Server className="w-4 h-4 sm:w-5 sm:h-5 text-orange-500 flex-shrink-0" />
                <span>Kredensial SSH VPS Linux</span>
              </h2>
              <p className="text-xs sm:text-sm text-[#52525b] mt-1">
                Akses root diperlukan untuk menjalankan skrip reinstallation.
              </p>
            </div>

            <div>
              <label htmlFor="host" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                Alamat IP Publik VPS
              </label>
              <input
                id="host"
                name="host"
                type="text"
                required
                placeholder="103.150.60.10"
                value={host}
                onChange={(e) => setHost(e.target.value)}
                className="input-bone w-full px-3.5 py-2.5 text-sm font-mono"
              />
              <p className="text-[11px] sm:text-xs text-[#71717a] mt-1">
                Pastikan VPS berarsitektur KVM dan menggunakan OS Linux standar.
              </p>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-4">
              <div>
                <label htmlFor="ssh_user" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  User SSH
                </label>
                <input
                  id="ssh_user"
                  name="ssh_user"
                  type="text"
                  value={sshUser}
                  onChange={(e) => setSshUser(e.target.value)}
                  className="input-bone w-full px-3.5 py-2.5 text-sm font-mono"
                />
              </div>

              <div>
                <label htmlFor="ssh_port" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  Port SSH
                </label>
                <input
                  id="ssh_port"
                  name="ssh_port"
                  type="number"
                  value={sshPort}
                  onChange={(e) => setSshPort(e.target.value)}
                  className="input-bone w-full px-3.5 py-2.5 text-sm font-mono"
                />
              </div>
            </div>

            <div>
              <label htmlFor="ssh_pass" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                Password SSH Root
              </label>
              <div className="relative">
                <input
                  id="ssh_pass"
                  name="ssh_pass"
                  type={showSshPass ? 'text' : 'password'}
                  required
                  value={sshPass}
                  onChange={(e) => setSshPass(e.target.value)}
                  className="input-bone w-full px-3.5 py-2.5 pr-11 text-sm font-mono"
                />
                <button
                  type="button"
                  onClick={() => setShowSshPass(!showSshPass)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-[#71717a] hover:text-[#0f172a] p-1"
                >
                  {showSshPass ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
              <p className="text-[11px] sm:text-xs text-[#71717a] mt-1">
                Password dienkripsi AES-256 dan otomatis dihapus saat proses selesai.
              </p>
            </div>

            <div className="pt-3 sm:pt-4 flex flex-col-reverse sm:flex-row sm:items-center justify-between gap-2.5 sm:gap-4">
              <button
                type="button"
                onClick={handleBack}
                className="w-full sm:w-auto px-4 py-2.5 rounded-lg border border-[#e8e4dc] text-xs font-bold text-[#52525b] hover:bg-[#f4f1ea] flex items-center justify-center gap-1.5"
              >
                <ArrowLeft className="w-4 h-4" />
                <span>Kembali</span>
              </button>
              <button
                type="button"
                onClick={handleNext}
                className="btn-orange w-full sm:w-auto px-6 py-2.5 rounded-lg font-bold text-sm flex items-center justify-center gap-2"
              >
                <span>Lanjutkan</span>
                <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}

        {/* STEP 3: WINDOWS CONFIG */}
        {step === 3 && (
          <div className="space-y-4 sm:space-y-5">
            <div>
              <h2 className="text-base sm:text-lg font-bold text-[#0f172a] flex items-center gap-2">
                <Layers className="w-4 h-4 sm:w-5 sm:h-5 text-orange-500 flex-shrink-0" />
                <span>Pilih Edisi & Kredensial Windows</span>
              </h2>
              <p className="text-xs sm:text-sm text-[#52525b] mt-1">
                Tentukan versi Windows dan buat password administrator baru untuk RDP Anda.
              </p>
            </div>

            <div>
              <label htmlFor="edition" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-2">
                Pilih Edisi Windows
              </label>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 sm:gap-2.5">
                {EDITIONS.map((ed) => (
                  <button
                    key={ed.key}
                    type="button"
                    onClick={() => setEdition(ed.key)}
                    className={`p-3 rounded-xl border text-left transition-all min-w-0 ${
                      edition === ed.key
                        ? 'border-orange-500 bg-orange-50/70 shadow-xs'
                        : 'border-[#e8e4dc] bg-white hover:border-[#18181b]'
                    }`}
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-bold text-xs sm:text-sm text-[#0f172a] truncate">{ed.label}</span>
                      <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[#f4f1ea] text-[#71717a] flex-shrink-0">
                        RAM {ed.min_ram_gb}GB+
                      </span>
                    </div>
                    <p className="text-[11px] text-[#71717a] mt-1 line-clamp-1">{ed.note}</p>
                  </button>
                ))}
              </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-4">
              <div>
                <label htmlFor="lang" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  Bahasa Tampilan
                </label>
                <select
                  id="lang"
                  name="lang"
                  value={lang}
                  onChange={(e) => setLang(e.target.value)}
                  className="input-bone w-full px-3.5 py-2.5 text-sm"
                >
                  <option value="en-us">English (US)</option>
                  <option value="zh-cn">Chinese Simplified (zh-cn)</option>
                </select>
              </div>

              <div>
                <label htmlFor="rdp_port" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  Port RDP
                </label>
                <input
                  id="rdp_port"
                  name="rdp_port"
                  type="number"
                  value={rdpPort}
                  onChange={(e) => setRdpPort(e.target.value)}
                  className="input-bone w-full px-3.5 py-2.5 text-sm font-mono"
                />
              </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-4">
              <div>
                <label htmlFor="rdp_user" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  User Administrator RDP
                </label>
                <input
                  id="rdp_user"
                  name="rdp_user"
                  type="text"
                  value={rdpUser}
                  onChange={(e) => setRdpUser(e.target.value)}
                  className="input-bone w-full px-3.5 py-2.5 text-sm font-mono"
                />
              </div>

              <div>
                <label htmlFor="rdp_pass" className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  Password RDP Baru
                </label>
                <div className="relative">
                  <input
                    id="rdp_pass"
                    name="rdp_pass"
                    type={showRdpPass ? 'text' : 'password'}
                    required
                    placeholder="Min 8 karakter"
                    value={rdpPass}
                    onChange={(e) => setRdpPass(e.target.value)}
                    className="input-bone w-full px-3.5 py-2.5 pr-11 text-sm font-mono"
                  />
                  <button
                    type="button"
                    onClick={() => setShowRdpPass(!showRdpPass)}
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-[#71717a] hover:text-[#0f172a] p-1"
                  >
                    {showRdpPass ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                  </button>
                </div>
              </div>
            </div>

            <div className="pt-3 sm:pt-4 flex flex-col-reverse sm:flex-row sm:items-center justify-between gap-2.5 sm:gap-4">
              <button
                type="button"
                onClick={handleBack}
                className="w-full sm:w-auto px-4 py-2.5 rounded-lg border border-[#e8e4dc] text-xs font-bold text-[#52525b] hover:bg-[#f4f1ea] flex items-center justify-center gap-1.5"
              >
                <ArrowLeft className="w-4 h-4" />
                <span>Kembali</span>
              </button>
              <button
                type="button"
                onClick={handleNext}
                className="btn-orange w-full sm:w-auto px-6 py-2.5 rounded-lg font-bold text-sm flex items-center justify-center gap-2"
              >
                <span>Review & Konfirmasi</span>
                <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}

        {/* STEP 4: REVIEW & DEPLOY */}
        {step === 4 && (
          <div className="space-y-5 sm:space-y-6">
            <div>
              <h2 className="text-base sm:text-lg font-bold text-[#0f172a] flex items-center gap-2">
                <CheckCircle2 className="w-4 h-4 sm:w-5 sm:h-5 text-orange-500 flex-shrink-0" />
                <span>Review Data Sebelum Pemasangan</span>
              </h2>
              <p className="text-xs sm:text-sm text-[#52525b] mt-1">
                Periksa kembali data di bawah. Proses ini akan menginstal ulang OS pada disk utama VPS Anda.
              </p>
            </div>

            <div className="rounded-xl border border-[#e8e4dc] bg-[#fbfaf7] divide-y divide-[#e8e4dc] overflow-hidden">
              <div className="p-3 sm:p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-1 sm:gap-4 text-xs sm:text-sm">
                <span className="text-[#71717a] font-medium flex-shrink-0">Kode Voucer</span>
                <span className="font-mono font-bold text-[#0f172a] bg-orange-100/60 px-2 py-0.5 rounded text-orange-800 break-all sm:text-right self-start sm:self-auto">
                  {voucher}
                </span>
              </div>
              <div className="p-3 sm:p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-1 sm:gap-4 text-xs sm:text-sm">
                <span className="text-[#71717a] font-medium flex-shrink-0">Target VPS</span>
                <span className="font-mono font-bold text-[#0f172a] break-all sm:text-right">
                  {sshUser}@{host}:{sshPort}
                </span>
              </div>
              <div className="p-3 sm:p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-1 sm:gap-4 text-xs sm:text-sm">
                <span className="text-[#71717a] font-medium flex-shrink-0">Edisi Windows</span>
                <span className="font-bold text-[#0f172a] break-all sm:text-right">
                  {selectedEditionObj.label} ({lang})
                </span>
              </div>
              <div className="p-3 sm:p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-1 sm:gap-4 text-xs sm:text-sm">
                <span className="text-[#71717a] font-medium flex-shrink-0">Kredensial RDP Nanti</span>
                <span className="font-mono text-xs text-[#0f172a] break-all sm:text-right">
                  User: <b>{rdpUser}</b> · Port: <b>{rdpPort}</b>
                </span>
              </div>
            </div>

            <div className="p-3.5 sm:p-4 rounded-xl bg-amber-50 border border-amber-200 text-amber-800 text-xs leading-relaxed space-y-1.5">
              <div className="font-bold flex items-center gap-1.5 text-amber-900">
                <HelpCircle className="w-4 h-4 text-amber-600 flex-shrink-0" />
                <span>Peringatan Penting:</span>
              </div>
              <p>• Seluruh data pada disk utama VPS akan diformat bersih.</p>
              <p>• Sesi SSH akan terputus saat VPS memulai reboot instalasi Windows (ini normal).</p>
              <p>• Estimasi durasi 20 hingga 50 menit tergantung kecepatan koneksi server ke upstream ISO.</p>
            </div>

            <div className="pt-2 flex flex-col-reverse sm:flex-row sm:items-center justify-between gap-2.5 sm:gap-4">
              <button
                type="button"
                disabled={loading}
                onClick={handleBack}
                className="w-full sm:w-auto px-4 py-2.5 rounded-lg border border-[#e8e4dc] text-xs font-bold text-[#52525b] hover:bg-[#f4f1ea] flex items-center justify-center gap-1.5 disabled:opacity-50"
              >
                <ArrowLeft className="w-4 h-4" />
                <span>Kembali</span>
              </button>
              <button
                type="button"
                disabled={loading}
                onClick={handleSubmit}
                className="btn-orange w-full sm:w-auto px-5 sm:px-8 py-3 rounded-lg font-extrabold text-xs sm:text-sm flex items-center justify-center gap-2 shadow-lg shadow-orange-500/30"
              >
                {loading ? (
                  <>
                    <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin flex-shrink-0" />
                    <span>Menghubungi VPS...</span>
                  </>
                ) : (
                  <>
                    <Terminal className="w-4 h-4 flex-shrink-0" />
                    <span className="sm:hidden">Mulai Pasang Windows</span>
                    <span className="hidden sm:inline">Mulai Pemasangan Windows Sekarang</span>
                  </>
                )}
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Info / FAQ Footer */}
      <div className="mt-6 sm:mt-8 card-bone p-4 sm:p-5 text-xs text-[#71717a] leading-relaxed">
        <div className="font-bold text-[#0f172a] mb-1.5 sm:mb-2 flex items-center gap-1.5">
          <Shield className="w-4 h-4 text-orange-500 flex-shrink-0" />
          <span>Keamanan & Jaminan Operasional</span>
        </div>
        <p>
          Installer bekerja langsung dari server ke server menggunakan skrip resmi <code className="font-mono text-[#0f172a]">reinstall.sh</code>. Password SSH Anda tidak disimpan ke database permanen dan otomatis dibersihkan saat Windows siap.
        </p>
      </div>
    </div>
  );
};
