import { type DragEvent } from 'react';

export default function NodePalette() {
    const onDragStart = (event: DragEvent, nodeType: string, label: string) => {
        event.dataTransfer.setData('application/reactflow/type', nodeType);
        event.dataTransfer.setData('application/reactflow/label', label);
        event.dataTransfer.effectAllowed = 'move';
    };

    return (
        <aside className="w-64 bg-gray-100 p-4 border-r border-gray-200 flex flex-col gap-4">
            <h2 className="font-bold text-gray-700">Nodes</h2>
            <div className="flex flex-col gap-2">
                <div
                    className="bg-white p-2 border border-blue-500 rounded cursor-move hover:shadow-md"
                    onDragStart={(event) => onDragStart(event, 'input', 'Start')}
                    draggable
                >
                    Manual Trigger
                </div>
                <div
                    className="bg-white p-2 border border-orange-500 rounded cursor-move hover:shadow-md"
                    onDragStart={(event) => onDragStart(event, 'input', 'Time Trigger')}
                    draggable
                >
                    Time Trigger
                </div>
                <div
                    className="bg-white p-2 border border-pink-500 rounded cursor-move hover:shadow-md"
                    onDragStart={(event) => onDragStart(event, 'input', 'Webhook Trigger')}
                    draggable
                >
                    Webhook Trigger
                </div>
                <div
                    className="bg-white p-2 border border-green-500 rounded cursor-move hover:shadow-md"
                    onDragStart={(event) => onDragStart(event, 'default', 'Log')}
                    draggable
                >
                    Console Log
                </div>
                <div
                    className="bg-white p-2 border border-purple-500 rounded cursor-move hover:shadow-md"
                    onDragStart={(event) => onDragStart(event, 'default', 'Set Data')}
                    draggable
                >
                    Set Data
                </div>
            </div>
        </aside>
    );
}
