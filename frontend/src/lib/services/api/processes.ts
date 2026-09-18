/** Süreç motoru servisi — şablonlar, gruplar, execution aksiyonları */

import { api } from './client';
import type {
	ExecutionRow,
	GroupDetailDto,
	ProcessEventDto,
	ProcessGroupDto,
	ProcessTemplateDto
} from '$lib/types';

export interface CreateTemplateInput {
	name: string;
	code: string;
	description?: string;
}

export interface CreateGroupInput {
	name: string;
	description?: string;
	steps: { template_id: string; required?: boolean }[];
	/** 0 tabanlı adım indeks çiftleri */
	dependencies: { step: number; depends_on: number }[];
}

export type TransitionAction =
	| 'start'
	| 'pause'
	| 'resume'
	| 'complete'
	| 'block'
	| 'unblock'
	| 'cancel';

export const processTemplateService = {
	list(): Promise<ProcessTemplateDto[]> {
		return api.get('/process-templates');
	},
	create(input: CreateTemplateInput): Promise<ProcessTemplateDto> {
		return api.post('/process-templates', input);
	}
};

export const processGroupService = {
	list(): Promise<ProcessGroupDto[]> {
		return api.get('/process-groups');
	},
	create(input: CreateGroupInput): Promise<ProcessGroupDto> {
		return api.post('/process-groups', input);
	},
	get(id: string): Promise<GroupDetailDto> {
		return api.get(`/process-groups/${id}`);
	}
};

export const processExecutionService = {
	listByWorkItem(workItemId: string): Promise<{ work_item_id: string; executions: ExecutionRow[] }> {
		return api.get(`/process-executions?work_item_id=${workItemId}`);
	},

	get(executionId: string): Promise<ExecutionRow> {
		return api.get(`/process-executions/${executionId}`);
	},

	events(executionId: string): Promise<ProcessEventDto[]> {
		return api.get(`/process-executions/${executionId}/events`);
	},

	assignGroup(workItemId: string, processGroupId: string): Promise<unknown> {
		return api.post(`/work-items/${workItemId}/assign-process-group`, { process_group_id: processGroupId });
	},

	assign(executionId: string, target: { user_id?: string; team_id?: string }): Promise<ExecutionRow> {
		return api.post(`/process-executions/${executionId}/assign`, target);
	},

	/** Durum aksiyonu: start/pause/resume/complete/block/unblock/cancel */
	act(
		executionId: string,
		action: TransitionAction,
		note?: string
	): Promise<{ execution: ExecutionRow; promoted: string[] }> {
		return api.post(`/process-executions/${executionId}/${action}`, note ? { note } : undefined);
	}
};
