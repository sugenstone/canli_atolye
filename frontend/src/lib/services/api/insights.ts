/** Faz 6 insight servisi: dashboard, matris, üretim akışı, aktivite */

import { api } from './client';
import type { SectionDto } from '$lib/types';

export interface WorkItemSummary {
	total: number;
	completed: number;
	in_progress: number;
	blocked: number;
	pending: number;
	late: number;
	completed_today: number;
}

export interface ProcessSummary {
	template_id: string;
	template_name: string;
	total: number;
	completed: number;
	in_progress: number;
	blocked: number;
}

export interface DashboardSummary {
	work_items: WorkItemSummary;
	processes: ProcessSummary[];
}

export interface MatrixCell {
	status: string;
	item_count: number;
	label: string;
	section_id: string;
	late: boolean;
}

export interface MatrixResponse {
	parent: SectionDto;
	rows: SectionDto[];
	cols: { position: number; label: string }[];
	cells: Record<string, Record<string, MatrixCell>>;
}

export interface FlowCard {
	execution_id: string;
	template_name: string;
	status: string;
	work_item_id: string;
	work_item_name: string;
	section_id: string;
	section_path: string;
	planned_end_at?: string | null;
}

export interface ActivityRow {
	timestamp: string;
	event_type: string;
	user_name: string;
	work_item_name: string;
	section_path: string;
	note?: string | null;
}

export const insightService = {
	dashboardSummary(projectId: string): Promise<DashboardSummary> {
		return api.get(`/projects/${projectId}/dashboard-summary`);
	},

	matrix(projectId: string, parentId: string): Promise<MatrixResponse> {
		return api.get(`/projects/${projectId}/matrix?parent=${parentId}`);
	},

	flow(projectId: string): Promise<FlowCard[]> {
		return api.get(`/projects/${projectId}/flow`);
	},

	activity(projectId: string, limit = 50): Promise<ActivityRow[]> {
		return api.get(`/projects/${projectId}/activity?limit=${limit}`);
	}
};

/** Plan tarihi güncelle (ADMIN/PM). */
export function setPlannedDates(
	executionId: string,
	dates: { planned_start_at?: string | null; planned_end_at?: string | null }
): Promise<unknown> {
	return api.patch(`/process-executions/${executionId}/planned-dates`, dates);
}

/** Şablon bazlı toplu plan (proje kapsamındaki aktif execution'lara). */
export function bulkPlan(
	projectId: string,
	input: { template_id: string; planned_end_at: string | null }
): Promise<{ updated: number }> {
	return api.post(`/projects/${projectId}/process-executions/bulk-plan`, input);
}

/** Süre kırılımı (aktif/duraklatılmış/bloke/toplam). */
export interface Durations {
	active_seconds: number;
	paused_seconds: number;
	blocked_seconds: number;
	lead_seconds: number | null;
	waiting_seconds: number | null;
}
export function durations(executionId: string): Promise<Durations> {
	return api.get(`/process-executions/${executionId}/durations`);
}
