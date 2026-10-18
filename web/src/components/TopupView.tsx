import React, { useState, useEffect } from 'react';
import { UserProfile } from '../types';
import { Wallet, QrCode, AlertCircle, CheckCircle2, ArrowRight, ExternalLink, X } from 'lucide-react';

interface TopupViewProps {
  user: UserProfile;
  onSuccess: () => void;
}

export const TopupView: React.FC<TopupViewProps> = ({ user, onSuccess }) => {
  const [amount, setAmount] = useState<number>(15000);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string>('');
  const [invoice, setInvoice] = useState<any>(null);
  const [invoiceStatus, setInvoiceStatus] = useState<string>('pending');

  const presets = [15000, 25000, 50000, 100000];

  const handleCreateInvoice = async () => {
    if (amount < 5000) {
      setError('Nominal minimal topup adalah Rp5.000');
      return;
    }
    setLoading(true);
    setError('');

    try {
      const res = await fetch('/topup', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: new URLSearchParams({ amount: String(amount) }).toString(),
      });

      if (res.redirected) {
        const ref = res.url.split('/topup/')[1];
        setInvoice({ reference: ref, amount });
        setInvoiceStatus('pending');
      } else {
        const text = await res.text();
        const m = text.match(/class="alert">([^<]+)/);
        setError(m ? m[1].trim() : 'Gagal membuat tagihan QRIS.');
      }
    } catch (err: any) {
      setError(err?.message || 'Gagal menghubungi server.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!invoice || invoiceStatus !== 'pending') return;

    const interval = setInterval(async () => {
      try {
        const res = await fetch(`/topup/${invoice.reference}/status`);
        if (res.ok) {
          const text = await res.text();
          if (text.includes('badge ok') || text.includes('Pembayaran sukses') || text.includes('paid')) {
            setInvoiceStatus('paid');
            clearInterval(interval);
            onSuccess();
          } else if (text.includes('expired') || text.includes('cancelled')) {
            setInvoiceStatus('expired');
            clearInterval(interval);
          }
        }
      } catch {
        // Ignore
      }
    }, 4000);

    return () => clearInterval(interval);
  }, [invoice, invoiceStatus, onSuccess]);

  return (
    <div className="max-w-xl w-full mx-auto space-y-5 sm:space-y-6">
      <div className="card-bone p-4 sm:p-7">
        <div className="flex items-center justify-between gap-3 mb-4 pb-4 border-b border-[#e8e4dc]">
          <div className="min-w-0">
            <h1 className="text-base sm:text-lg font-bold text-[#0f172a] flex items-center gap-2 truncate">
              <QrCode className="w-5 h-5 text-orange-500 flex-shrink-0" />
              <span>Isi Saldo via QRIS</span>
            </h1>
            <p className="text-xs text-[#71717a] mt-0.5 truncate">GoPay Merchant (GoBiz) · Real-time 24 Jam</p>
          </div>
          <div className="text-right flex-shrink-0">
            <span className="text-[11px] text-[#71717a] block">Saldo Anda</span>
            <span className="text-sm font-bold text-[#0f172a]">Rp{user.balance.toLocaleString('id-ID')}</span>
          </div>
        </div>

        {error && (
          <div className="mb-5 p-3.5 rounded-xl bg-red-50 border border-red-200 text-red-700 text-xs flex items-center gap-2.5">
            <AlertCircle className="w-4 h-4 text-red-500 flex-shrink-0" />
            <span>{error}</span>
          </div>
        )}

        {invoice ? (
          <div className="text-center space-y-4 py-2">
            <div className="p-3 sm:p-4 rounded-xl bg-[#f4f1ea] border border-[#e2ded4] inline-block max-w-full">
              {invoiceStatus === 'paid' ? (
                <div className="py-6 px-4 space-y-2 text-center text-emerald-800">
                  <CheckCircle2 className="w-10 h-10 text-emerald-600 mx-auto" />
                  <p className="font-bold text-sm">Pembayaran Berhasil Diterima!</p>
                  <p className="text-xs">Saldo Anda telah ditambahkan otomatis.</p>
                </div>
              ) : (
                <>
                  <p className="text-xs font-bold text-[#71717a] uppercase mb-2">Scan QRIS Ini:</p>
                  <img
                    src={`https://pg-v1.jaystore.top/qr/PG-${invoice.reference}?format=png`}
                    alt="QRIS"
                    className="w-48 sm:w-56 h-48 sm:h-56 mx-auto rounded-lg bg-white p-2 border border-[#e2ded4] shadow-xs max-w-full object-contain"
                    onError={(e: any) => {
                      // Fallback if needed
                      e.target.src = `/topup/${invoice.reference}`;
                    }}
                  />
                  <p className="text-[11px] text-[#71717a] mt-2">
                    Total: <b className="text-[#0f172a]">Rp{amount.toLocaleString('id-ID')}</b>
                  </p>
                </>
              )}
            </div>

            {invoiceStatus === 'pending' && (
              <p className="text-xs text-[#71717a] flex items-center justify-center gap-2">
                <span className="w-2 h-2 rounded-full bg-orange-500 animate-ping" />
                <span>Menunggu verifikasi pembayaran otomatis (cek tiap 4s)...</span>
              </p>
            )}

            <div>
              <button
                type="button"
                onClick={() => setInvoice(null)}
                className="text-xs font-bold text-[#71717a] hover:text-[#0f172a] underline"
              >
                Kembali / Pilih Nominal Lain
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-5">
            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-2">
                Pilih Nominal Cepat
              </label>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
                {presets.map((val) => (
                  <button
                    key={val}
                    type="button"
                    onClick={() => setAmount(val)}
                    className={`py-2.5 px-3 rounded-xl border text-xs font-bold transition-all ${
                      amount === val
                        ? 'border-orange-500 bg-orange-50 text-orange-700 shadow-xs'
                        : 'border-[#e8e4dc] bg-white text-[#0f172a] hover:border-[#18181b]'
                    }`}
                  >
                    Rp{val.toLocaleString('id-ID')}
                  </button>
                ))}
              </div>
            </div>

            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-2">
                Atau Masukkan Nominal Khusus
              </label>
              <div className="relative">
                <span className="absolute left-3.5 top-1/2 -translate-y-1/2 font-bold text-xs text-[#71717a]">
                  Rp
                </span>
                <input
                  type="number"
                  min="5000"
                  max="5000000"
                  step="1000"
                  value={amount}
                  onChange={(e) => setAmount(Number(e.target.value))}
                  className="input-bone w-full pl-10 pr-4 py-2.5 text-sm font-bold font-mono"
                />
              </div>
              <p className="text-[11px] text-[#71717a] mt-1.5">Minimal Rp5.000 · Bebas biaya transaksi gateway.</p>
            </div>

            <div className="pt-2">
              <button
                type="button"
                disabled={loading}
                onClick={handleCreateInvoice}
                className="btn-orange w-full py-3 rounded-xl text-sm font-bold flex items-center justify-center gap-2"
              >
                {loading ? 'Menyiapkan QRIS...' : 'Lanjutkan ke Pembayaran QRIS'}
                <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
