import React from 'react';
import { X, Shield, Lock } from 'lucide-react';

interface LoginModalProps {
  isOpen: boolean;
  onClose: () => void;
  botUsername: string;
}

export const LoginModal: React.FC<LoginModalProps> = ({
  isOpen,
  onClose,
  botUsername,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 backdrop-blur-xs">
      <div className="card-bone w-full max-w-sm p-6 relative animate-in fade-in zoom-in-95 duration-200">
        <button
          onClick={onClose}
          className="absolute right-4 top-4 text-[#71717a] hover:text-[#0f172a] p-1 rounded-lg"
        >
          <X className="w-4 h-4" />
        </button>

        <div className="text-center space-y-3 mb-6">
          <div className="w-10 h-10 rounded-full bg-orange-100 text-orange-600 flex items-center justify-center mx-auto">
            <Lock className="w-5 h-5" />
          </div>
          <h2 className="text-base font-bold text-[#0f172a]">Masuk ke Akun Anda</h2>
          <p className="text-xs text-[#71717a]">
            Masuk dengan aman via Telegram Widget. Tidak ada password akun yang disimpan.
          </p>
        </div>

        <div className="text-center py-2">
          {botUsername ? (
            <div className="inline-block">
              <a
                href={`https://oauth.telegram.org/auth?bot_id=${botUsername}&origin=${encodeURIComponent(
                  window.location.origin
                )}&embed=1&request_access=write&return_to=${encodeURIComponent(
                  window.location.origin + '/auth/telegram'
                )}`}
                className="btn-orange px-5 py-2.5 rounded-lg text-xs font-bold inline-flex items-center gap-2"
              >
                <span>Buka Login Telegram</span>
              </a>
            </div>
          ) : (
            <p className="text-xs text-red-600">Telegram Bot belum terkonfigurasi.</p>
          )}

          {/* Dev login fallback */}
          <div className="mt-4 pt-4 border-t border-[#e8e4dc]">
            <form method="post" action="/auth/dev">
              <button
                type="submit"
                className="text-xs text-[#71717a] hover:text-[#0f172a] underline"
              >
                Masuk sebagai Dev (hanya di mode pengujian)
              </button>
            </form>
          </div>
        </div>
      </div>
    </div>
  );
};
