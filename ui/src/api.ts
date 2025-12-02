import { type NodeType } from './types';

export const fetchNodeTypes = async (): Promise<NodeType[]> => {
    const response = await fetch('/api/node-types');
    if (!response.ok) {
        throw new Error('Failed to fetch node types');
    }
    return response.json();
};

export const runWorkflow = async (workflow: any): Promise<any> => {
    const response = await fetch('/api/run', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ workflow: JSON.stringify(workflow) }),
    });

    if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to run workflow');
    }

    return response.json();
};

export const getJobStatus = async (id: string): Promise<any> => {
    const response = await fetch(`/api/jobs/${id}`);
    if (!response.ok) {
        throw new Error('Failed to fetch job status');
    }
    return response.json();
};
