import React, { useState, useEffect } from 'react';
import { UserProfile } from './types';
import { Navbar } from './components/Navbar';
import { GuestWizard } from './components/GuestWizard';
import { JobMonitor } from './components/JobMonitor';
import { DashboardView } from './components/DashboardView';
import { TopupView } from './components/TopupView';
import { AdminPanel } from './components/AdminPanel';
import { LoginModal } from './components/LoginModal';

export const App: React.FC = () => {
  const [currentTab, setTab] = useState<string>('guest');
  const [user, setUser] = useState<UserProfile | null>(null);
  const [activeToken, setActiveToken] = useState<string | null>(null);
  const [activeJobId, setActiveJobId] = useState<number | null>(null);
  const [isLoginModalOpen, setLoginModalOpen] = useState<boolean>(false);
  const [savedGuestJob, setSavedGuestJob] = useState<{
    token: string;
    host: string;
    voucher: string;
    os_label?: string;
    time: number;
  } | null>(null);

  // Check URL routing on mount
  useEffect(() => {
    // Check saved guest job from localStorage
    try {
      const raw = localStorage.getItem('rdpi_saved_guest_job');
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Date.now() - parsed.time < 24 * 3600 * 1000) {
          setSavedGuestJob(parsed);
        } else {
          localStorage.removeItem('rdpi_saved_guest_job');
        }
      }
    } catch {}

    const path = window.location.pathname;
    if (path.startsWith('/g/')) {
      const token = path.replace('/g/', '').split('/')[0];
      if (token) setActiveToken(token);
    } else if (path.startsWith('/j/')) {
      const id = parseInt(path.replace('/j/', '').split('/')[0], 10);
      if (!isNaN(id)) setActiveJobId(id);
    } else if (path === '/topup') {
      setTab('topup');
    } else if (path === '/dashboard') {
      setTab('dashboard');
    } else if (path.startsWith('/admin')) {
      setTab('admin');
    }

    // Check user auth session
    fetch('/dashboard', { headers: { Accept: 'application/json' } })
      .then((r) => {
        if (r.ok) return r.json();
        return null;
      })
      .then((data) => {
        if (data?.user) {
          setUser({
            id: data.user.id,
            username: data.user.username,
            first_name: data.user.first_name,
            last_name: data.user.last_name,
            balance: data.balance || 0,
            is_admin: data.is_admin || false,
          });
        }
      })
      .catch(() => {});
  }, []);

  const handleLogout = async () => {
    await fetch('/keluar', { method: 'POST' });
    setUser(null);
    setTab('guest');
    window.location.href = '/';
  };

  return (
    <div className="min-h-screen flex flex-col bg-[#fbfaf7] text-[#0f172a]">
      {/* Navigation Bar */}
      <Navbar
        currentTab={currentTab}
        setTab={(tab) => {
          setActiveToken(null);
          setActiveJobId(null);
          setTab(tab);
        }}
        user={user}
        onLoginClick={() => setLoginModalOpen(true)}
        onLogout={handleLogout}
      />

      {/* Main Content Area */}
      <main className="flex-1 max-w-5xl w-full mx-auto px-3 sm:px-6 py-5 sm:py-12">
        {savedGuestJob && !activeToken && !activeJobId && currentTab === 'guest' && (
          <div className="mb-6 p-4 rounded-xl bg-orange-50 border border-orange-200 text-orange-950 flex flex-col sm:flex-row sm:items-center justify-between gap-3 shadow-xs">
            <div className="text-xs">
              <span className="font-bold flex items-center gap-1.5 text-orange-900">
                <span className="w-2 h-2 rounded-full bg-orange-500 animate-ping inline-block" />
                Instalasi Terakhir Anda Masih Berlangsung
              </span>
              <p className="text-[#52525b] mt-0.5">
                Target: <b>{savedGuestJob.host}</b> ({savedGuestJob.os_label || 'Windows'}) · Voucer: <span className="font-mono font-bold">{savedGuestJob.voucher}</span>
              </p>
            </div>
            <div className="flex items-center gap-2 flex-shrink-0">
              <button
                type="button"
                onClick={() => {
                  setActiveToken(savedGuestJob.token);
                  window.history.pushState({}, '', `/g/${savedGuestJob.token}`);
                }}
                className="btn-orange px-3.5 py-1.5 rounded-lg text-xs font-bold"
              >
                Buka Progres
              </button>
              <button
                type="button"
                onClick={() => {
                  setSavedGuestJob(null);
                  localStorage.removeItem('rdpi_saved_guest_job');
                }}
                className="p-1.5 text-[#71717a] hover:text-[#0f172a] text-xs font-bold"
              >
                Abaikan
              </button>
            </div>
          </div>
        )}

        {activeToken ? (
          <JobMonitor
            token={activeToken}
            onBack={() => {
              setActiveToken(null);
              window.history.pushState({}, '', '/');
            }}
          />
        ) : activeJobId ? (
          <JobMonitor
            jobId={activeJobId}
            onBack={() => {
              setActiveJobId(null);
              window.history.pushState({}, '', '/dashboard');
            }}
          />
        ) : currentTab === 'guest' ? (
          <GuestWizard
            onSuccess={(token) => {
              setActiveToken(token);
              window.history.pushState({}, '', `/g/${token}`);
            }}
          />
        ) : currentTab === 'dashboard' && user ? (
          <DashboardView
            user={user}
            onTopupClick={() => setTab('topup')}
            onSelectJob={(id) => setActiveJobId(id)}
            onNewInstallClick={() => setTab('guest')}
          />
        ) : currentTab === 'topup' && user ? (
          <TopupView
            user={user}
            onSuccess={() => {
              // Refresh balance
              fetch('/dashboard', { headers: { Accept: 'application/json' } })
                .then((r) => r.json())
                .then((data) => {
                  if (data?.balance !== undefined) {
                    setUser((prev) => (prev ? { ...prev, balance: data.balance } : null));
                  }
                });
            }}
          />
        ) : currentTab === 'admin' ? (
          <AdminPanel />
        ) : (
          <GuestWizard
            onSuccess={(token) => {
              setActiveToken(token);
              window.history.pushState({}, '', `/g/${token}`);
            }}
          />
        )}
      </main>

      {/* Footer */}
      <footer className="border-t border-[#e8e4dc] bg-[#f4f1ea]/60 py-6 px-4 text-center text-xs text-[#71717a]">
        <div className="max-w-5xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-3">
          <span>rdpinstall · Instalasi Windows di VPS Anda Sendiri</span>
          <span className="font-mono text-[11px] text-[#a1a1aa]">
            Universal KVM Deployment · Pure Open Source reinstall.sh
          </span>
        </div>
      </footer>

      {/* Login Modal */}
      <LoginModal
        isOpen={isLoginModalOpen}
        onClose={() => setLoginModalOpen(false)}
        botUsername="InstallXJayBot"
      />
    </div>
  );
};

export default App;
