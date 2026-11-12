# frozen_string_literal: true

module Api
  module V1
    class PlotsController < ApplicationController
      def index
        plots = Plot.includes(field: :farm).order(:name)
        render json: plots.as_json(include: { field: { include: :farm } })
      end

      def show
        render json: Plot.find(params[:id]).as_json(include: { field: { include: :farm } })
      end

      def create
        plot = Plot.new(plot_params)
        if plot.save
          AuditEvent.create!(action: "create", resource_type: "Plot", resource_id: plot.id)
          render json: plot, status: :created
        else
          render json: { errors: plot.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def plot_params
        params.require(:plot).permit(:field_id, :name, :area_hectares)
      end
    end
  end
end
