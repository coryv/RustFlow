import { type Node } from 'reactflow';

interface PropertiesPanelProps {
    selectedNode: Node | null;
    setNodes: React.Dispatch<React.SetStateAction<Node[]>>;
}

export default function PropertiesPanel({ selectedNode, setNodes }: PropertiesPanelProps) {
    if (!selectedNode) {
        return (
            <aside className="w-64 bg-gray-100 p-4 border-l border-gray-200">
                <div className="text-gray-500 text-sm">Select a node to edit properties</div>
            </aside>
        );
    }

    const onChange = (evt: React.ChangeEvent<HTMLInputElement>) => {
        const { name, value } = evt.target;
        setNodes((nds) =>
            nds.map((n) => {
                if (n.id === selectedNode.id) {
                    return {
                        ...n,
                        data: {
                            ...n.data,
                            [name]: value,
                        },
                    };
                }
                return n;
            })
        );
    };

    return (
        <aside className="w-64 bg-gray-100 p-4 border-l border-gray-200 flex flex-col gap-4">
            <h2 className="font-bold text-gray-700">Properties</h2>
            <div className="flex flex-col gap-1">
                <label className="text-xs font-semibold text-gray-500">Label</label>
                <input
                    type="text"
                    name="label"
                    value={selectedNode.data.label}
                    onChange={onChange}
                    className="border border-gray-300 rounded px-2 py-1 text-sm"
                />
            </div>

            {selectedNode.data.label.includes('Time') && (
                <div className="flex flex-col gap-1">
                    <label className="text-xs font-semibold text-gray-500">Cron Expression</label>
                    <input
                        type="text"
                        name="cron"
                        placeholder="* * * * *"
                        value={selectedNode.data.cron || ''}
                        onChange={onChange}
                        className="border border-gray-300 rounded px-2 py-1 text-sm font-mono"
                    />
                </div>
            )}

            {selectedNode.data.label.includes('Webhook') && (
                <>
                    <div className="flex flex-col gap-1">
                        <label className="text-xs font-semibold text-gray-500">Path</label>
                        <input
                            type="text"
                            name="path"
                            placeholder="/webhook"
                            value={selectedNode.data.path || ''}
                            onChange={onChange}
                            className="border border-gray-300 rounded px-2 py-1 text-sm"
                        />
                    </div>
                    <div className="flex flex-col gap-1">
                        <label className="text-xs font-semibold text-gray-500">Method</label>
                        <select
                            name="method"
                            value={selectedNode.data.method || 'POST'}
                            onChange={onChange as any}
                            className="border border-gray-300 rounded px-2 py-1 text-sm"
                        >
                            <option value="GET">GET</option>
                            <option value="POST">POST</option>
                            <option value="PUT">PUT</option>
                        </select>
                    </div>
                </>
            )}

            <div className="text-xs text-gray-400 mt-2">
                ID: {selectedNode.id}
                <br />
                Type: {selectedNode.type}
            </div>
        </aside>
    );
}
