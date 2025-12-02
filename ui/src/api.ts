import { type NodeType } from './types';

export async function fetchNodeTypes(): Promise<NodeType[]> {
    const response = await fetch('/api/node-types');
    if (!response.ok) {
        throw new Error('Failed to fetch node types');
    }
    return response.json();
}
