# frozen_string_literal: true

module Api
  module V1
    class RolesController < ApplicationController
      def index
        render json: Role.order(:name)
      end

      def show
        render json: Role.find(params[:id])
      end

      def create
        role = Role.new(role_params)
        if role.save
          AuditEvent.create!(action: "create", resource_type: "Role", resource_id: role.id)
          render json: role, status: :created
        else
          render json: { errors: role.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def role_params
        params.require(:role).permit(:name, :key, :description)
      end
    end
  end
end
