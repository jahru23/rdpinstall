export interface WindowsEdition {
  key: string;
  label: string;
  note: string;
  min_ram_gb: number;
}

export const EDITIONS: WindowsEdition[] = [
  { key: "srv2022", label: "Server 2022", note: "Rekomendasi server, stabil & cepat", min_ram_gb: 1 },
  { key: "srv2019", label: "Server 2019", note: "Populer untuk RDP dan multi-tool", min_ram_gb: 1 },
  { key: "srv2025", label: "Server 2025", note: "Versi server terbaru (RAM min 2GB)", min_ram_gb: 2 },
  { key: "srv2016", label: "Server 2016", note: "Paling hemat RAM, sangat ringan", min_ram_gb: 1 },
  { key: "w11ltsc24", label: "Windows 11 LTSC 2024", note: "Tanpa bloatware, paling bersih", min_ram_gb: 2 },
  { key: "w11pro", label: "Windows 11 Pro", note: "Fitur lengkap Windows 11 (RAM min 2GB)", min_ram_gb: 2 },
  { key: "w10ltsc21", label: "Windows 10 LTSC 2021", note: "Sangat stabil & minim update", min_ram_gb: 1 },
  { key: "w10pro", label: "Windows 10 Pro", note: "Edisi desktop paling umum", min_ram_gb: 1 },
];

export interface JobEvent {
  ts: number;
  phase: string;
  message: string;
}

export interface JobData {
  id: number;
  status: 'queued' | 'running' | 'done' | 'failed' | 'cancelled';
  phase: string;
  progress: number;
  os_label: string;
  host: string;
  rdp_port: number;
  rdp_user: string;
  ssh_port: number;
  ssh_user: string;
  guest: number;
  voucher_code: string;
  edition: string;
  lang: string;
  error?: string;
  created_at: number;
  done_at: number;
  expires_at: number;
  wiped: number;
  password?: string;
  events: JobEvent[];
}

export interface UserProfile {
  id: number;
  tg_id?: number;
  username: string;
  first_name: string;
  last_name: string;
  balance: number;
  is_admin: boolean;
}

export interface VoucherItem {
  code: string;
  created_at: number;
  used_at: number;
  used_job_id: number;
  used_host: string;
  revoked: number;
  note: string;
}
