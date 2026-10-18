import React, { useState, useEffect, useRef } from 'react';
import { UserProfile } from '../types';
import { QrCode, AlertCircle, CheckCircle2, ArrowRight, ExternalLink } from 'lucide-react';

type PaymentStatus = {
  status: string;
  balance: number;
  fee_note: string;
  payment: { qr_string: string; pay_url: string; amount: number; total_amount: number; expires_at: number };
};

const isPaid = (status: unknown) => status === 'paid';
const isExpired = (status: unknown) => ['expired', 'cancelled', 'failed'].includes(String(status));
const getErrorMessage = (body: string) => {
  try { const data = JSON.parse(body); return data?.error?.message || data?.error || 'Gagal membuat tagihan QRIS.'; }
  catch { return 'Gagal membuat tagihan QRIS.'; }
};
const qrImageUrl = (raw: string) => {
  if (!raw) return '';
  if (raw.startsWith('http://') || raw.startsWith('https://')) return raw;
  return `https://pg-v1.jaystore.top${raw.startsWith('/') ? '' : '/'}${raw}`;
};

export const topupStatusHelpers = { isPaid, isExpired, qrImageUrl, getErrorMessage };

interface TopupViewProps {
  user: UserProfile;
  onSuccess: () => void;
}

export const TopupView: React.FC<TopupViewProps> = ({ user, onSuccess }) => {
  const [amount, setAmount] = useState<number>(10000);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string>('');
  const [invoice, setInvoice] = useState<any>(null);
  const [payment, setPayment] = useState<PaymentStatus | null>(null);
  const [invoiceStatus, setInvoiceStatus] = useState<string>('pending');
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const presets = [2000, 10000, 20000, 50000];

  const refreshPayment = async (reference: string) => {
    const res = await fetch(`/topup/${encodeURIComponent(reference)}/status`, {
      headers: { Accept: 'application/json' }, cache: 'no-store',
    });
    if (!res.ok) throw new Error(`Gagal memeriksa status tagihan (${res.status}).`);
    const data = await res.json();
    const next: PaymentStatus = data;
    setPayment(next);
    if (isPaid(next.status)) {
      setInvoiceStatus('paid');
      onSuccess();
    } else if (isExpired(next.status)) {
      setInvoiceStatus('expired');
    }
    return next;
  };

  const handleCreateInvoice = async () => {
    if (amount < 2000) {
      setError('Nominal minimal topup adalah Rp2.000');
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
        const url = new URL(res.url, window.location.origin);
        const ref = decodeURIComponent(url.pathname.split('/').filter(Boolean).at(-1) || '');
        if (!ref || ref === 'masuk') {
          setError('Sesi login berakhir. Silakan masuk kembali.');
          return;
        }
        setInvoice({ reference: ref, amount });
        setPayment(null);
        setInvoiceStatus('pending');
        try { await refreshPayment(ref); } catch (err: any) { setError(err?.message || 'Gagal memuat tagihan.'); }
      } else if (res.ok) {
        const text = await res.text();
        if (res.url.includes('/topup/')) {
          const ref = decodeURIComponent(new URL(res.url).pathname.split('/').filter(Boolean).at(-1) || '');
          setInvoice({ reference: ref, amount });
          setInvoiceStatus('pending');
          try { await refreshPayment(ref); } catch (err: any) { setError(err?.message || 'Gagal memuat tagihan.'); }
        } else {
          setError(getErrorMessage(text));
        }
      } else {
        setError(getErrorMessage(await res.text()));
      }
    } catch (err: any) {
      setError(err?.message || 'Gagal menghubungi server.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!invoice || invoiceStatus !== 'pending') return;
    const poll = async () => {
      try { await refreshPayment(invoice.reference); }
      catch (err: any) { setError(err?.message || 'Gagal memeriksa status pembayaran.'); }
    };
    void poll();
    intervalRef.current = setInterval(() => { void poll(); }, 10_000);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
      intervalRef.current = null;
    };
  }, [invoice?.reference, invoiceStatus]);

  useEffect(() => {
    if (invoice) window.history.replaceState({}, '', `/topup/${encodeURIComponent(invoice.reference)}`);
    else if (window.location.pathname.startsWith('/topup/')) window.history.replaceState({}, '', '/topup');
  }, [invoice?.reference]);

  useEffect(() => {
    const pathRef = window.location.pathname.startsWith('/topup/')
      ? decodeURIComponent(window.location.pathname.split('/').filter(Boolean).at(-1) || '')
      : '';
    if (!pathRef) return;
    setInvoice({ reference: pathRef, amount });
    void refreshPayment(pathRef).catch((err: any) => setError(err?.message || 'Tagihan tidak ditemukan.'));
  }, []);

  return (
    <div className="max-w-xl w-full mx-auto space-y-5 sm:space-y-6">
      <div className="card-bone p-4 sm:p-7">
        <div className="flex items-center justify-between gap-3 mb-4 pb-4 border-b border-[#e8e4dc]">
          <div className="min-w-0">
            <h1 className="text-base sm:text-lg font-bold text-[#0f172a] flex items-center gap-2 truncate">
              <QrCode className="w-5 h-5 text-orange-500 flex-shrink-0" />
              <span>Isi Saldo via QRIS</span>
            </h1>
            <p className="text-xs text-[#71717a] mt-0.5 truncate">QRIS Otomatis (Semua Bank & E-Wallet) · Real-time 24 Jam</p>
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
            {invoiceStatus === 'paid' ? (
              <div className="p-5 rounded-xl bg-emerald-50 border border-emerald-200 text-emerald-900">
                <CheckCircle2 className="w-10 h-10 text-emerald-600 mx-auto" />
                <p className="font-bold text-sm mt-2">Pembayaran Berhasil Diterima</p>
                <p className="text-xs mt-1">Saldo akun: Rp{(payment?.balance ?? user.balance).toLocaleString('id-ID')}</p>
                <p className="text-xs mt-1">{payment?.fee_note || 'Saldo sudah dikreditkan.'}</p>
              </div>
            ) : (
              <div className="p-3 sm:p-4 rounded-xl bg-[#f4f1ea] border border-[#e2ded4] inline-block max-w-full">
                {payment?.payment?.qr_string ? (
                  <>
                    <p className="text-xs font-bold text-[#71717a] uppercase mb-2">Scan QRIS Ini:</p>
                    <img src={qrImageUrl(payment.payment.qr_string)} alt="QRIS" className="w-48 sm:w-56 h-48 sm:h-56 mx-auto rounded-lg bg-white p-2 border border-[#e2ded4] shadow-xs max-w-full object-contain" />
                    <p className="text-[11px] text-[#71717a] mt-2">Total: <b className="text-[#0f172a]">Rp{(payment.payment.total_amount || amount).toLocaleString('id-ID')}</b></p>
                    {payment.payment.expires_at > 0 && <p className="text-[11px] text-[#71717a] mt-1">Berlaku sampai {new Date(payment.payment.expires_at * 1000).toLocaleString('id-ID')}</p>}
                  </>
                ) : payment?.payment?.pay_url ? (
                  <div className="p-4 max-w-sm">
                    <AlertCircle className="w-8 h-8 text-orange-500 mx-auto" />
                    <p className="font-bold text-sm mt-2">QRIS tidak tersedia langsung</p>
                    <a className="inline-flex mt-3 text-sm font-bold text-orange-700 underline" href={payment.payment.pay_url} target="_blank" rel="noreferrer">Buka halaman pembayaran <ExternalLink className="w-4 h-4 ml-1" /></a>
                  </div>
                ) : (
                  <div className="p-5 max-w-sm"><AlertCircle className="w-8 h-8 text-orange-500 mx-auto" /><p className="font-bold text-sm mt-2">QR pembayaran belum tersedia</p><p className="text-xs text-[#71717a] mt-1">Status terus diperiksa. Jika masalah berlanjut, kembali dan buat invoice baru.</p></div>
                )}
              </div>
            )}
            {invoiceStatus === 'pending' && <p className="text-xs text-[#71717a] flex items-center justify-center gap-2"><span className="w-2 h-2 rounded-full bg-orange-500 animate-ping" />Menunggu konfirmasi gateway; status server dicek tiap 10 detik.</p>}
            {invoiceStatus === 'expired' && <p className="text-xs text-red-700">Invoice kedaluwarsa atau dibatalkan. Buat tagihan baru.</p>}
            {error && <p className="text-xs text-red-700">{error}</p>}
            <button type="button" onClick={() => { setInvoice(null); setPayment(null); setError(''); }} className="text-xs font-bold text-[#71717a] hover:text-[#0f172a] underline">Kembali / Pilih Nominal Lain</button>
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
                  min="2000"
                  max="5000000"
                  step="1000"
                  value={amount}
                  onChange={(e) => setAmount(Number(e.target.value))}
                  className="input-bone w-full pl-10 pr-4 py-2.5 text-sm font-bold font-mono"
                />
              </div>
              <p className="text-[11px] text-[#71717a] mt-1.5">Minimal Rp2.000 · Bebas biaya transaksi gateway.</p>
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
